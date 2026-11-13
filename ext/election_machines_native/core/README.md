# election-machines

> **AMS LeLann**, NCC-4206, *LeLann*-class command relay.

Engine-room flight manual: model command handovers through one lease authority.

`no_std` lease-backed owner nomination under one authoritative history.
Among participating candidates, the smallest numeric ID is nominated. An
unexpired incumbent remains leader even if absent from the current nomination
list; this list is not a consensus membership configuration.

All lease-machine persistence, fencing, authentication, and clock requirements
apply. A fresh election instance is a fresh namespace, not failover recovery.
Heartbeats renew a lease; they are not an asynchronous failure detector. This
policy is **not Raft, quorum voting, or partition-safe leader election without
an external serialized durable authority**. Raft elections remain in
`consensus-machines`.

## Contract

- **One refusal type.** Every call that can refuse returns
  `Result<_, Refusal>`, and a refusal changes nothing: no grant, no token, no
  observed clock. `Refusal` has two variants: `NoCandidates`, the election's
  own, and `Lease(LeaseRefusal)`, which carries the embedded
  `LeaseAuthority`'s refusal whole. `LeaseRefusal` is `lease_machines::Refusal`,
  re-exported; `Refusal: From<LeaseRefusal>`, and `Display` and `Error`
  delegate to it. The election copies none of the lease core's names, so it
  has no variant it cannot answer. The table below lists which lease refusals
  can arrive from each call; `Occupied`, `Held`, `Fenced` and `OutOfPhase`
  arrive from none.
- **A live incumbent is honoured, not refused.** `elect` answers
  `Ok(Leadership::Incumbent(lease))` with the lease in force, whoever the
  candidates were, and changes nothing. That is what the authority's `Occupied`
  and `Held` mean to an election, so `elect` never refuses with them.
  `Ok(Leadership::Elected(lease))` is a new leader under a new token.
- **An empty candidate list is a refusal**, `NoCandidates`, checked before the
  clock and whatever the incumbent: nobody was nominated, so nothing was
  honoured. Read the leader with `lease()`.
- **Repeats.** A repeated `heartbeat` is honoured and never shortens the
  lease. A repeated `resign` is `Lease(NoLease)`: the election keeps no record of who
  resigned what, so it cannot prove the claim.
- **Scopes.** The election issues no ids of its own: leadership tokens come
  from the authority's sequence and carry its scope. `Election::with_scope`
  picks it, `new()` is scope zero, and a token of another scope, or one never
  issued, is `Lease(Unknown)` before any lookup. Tokens of two scopes have no order:
  compare with `FencingToken::order` or `is_newer_than`.
- **Restarts.** A fresh election counts from one again. Persist `checkpoint()`
  before exposing a decision and rebuild with
  `Election::restored(scope, checkpoint, clock)`: the scope and the clock
  policy are stated again, and a checkpoint of another scope is `None`.
  Restoring an older checkpoint than the newest committed one reissues tokens;
  no pure value can detect that.
- **Time** is `core::time::Duration` since an epoch the caller picks, for
  `now`, `ttl` and `expires_at` alike. Under `ClockPolicy::Reject` an earlier
  `now` is `Lease(ClockRegressed)`; under `Clamp` the call is evaluated at the latest
  honoured observation.
- **Sibling types.** `OwnerId`, `FencingToken`, `Lease`, `Checkpoint` and
  `ClockPolicy` are `lease-machines` types, re-exported here, and so is its
  `Refusal`, as `LeaseRefusal`.

| Call | Honoured | Refused: own | Refused: `Lease(..)` carrying |
| --- | --- | --- | --- |
| `elect(candidates, now, ttl)` | `Leadership::Elected(lease)`, `Leadership::Incumbent(lease)` | `NoCandidates` | `ClockRegressed`, `InvalidTtl`, `Exhausted` |
| `heartbeat(owner, token, now, ttl)` | `Lease` (same token) | | `Unknown`, `ClockRegressed`, `NoLease`, `WrongOwner`, `StaleToken`, `Expired`, `InvalidTtl` |
| `resign(owner, token, now)` | `Lease` (the one given up) | | `Unknown`, `ClockRegressed`, `NoLease`, `WrongOwner`, `StaleToken`, `Expired` |
| `restored(scope, checkpoint, clock)` | `Some(Election)` | `None`: the checkpoint is of another scope | |

## Example

```rust
use core::time::Duration;
use election_machines::{ClockPolicy, Election, Leadership, LeaseRefusal, OwnerId, Refusal};

fn main() -> Result<(), Refusal> {
    let seconds = Duration::from_secs;
    let mut election = Election::with_scope(7);

    let candidates = [OwnerId::new(9), OwnerId::new(2)];
    let Leadership::Elected(lease) = election.elect(&candidates, seconds(0), seconds(30))? else {
        unreachable!("nobody led before");
    };
    assert_eq!((lease.owner(), lease.token().scope()), (OwnerId::new(2), 7));

    // A live leader stays, whoever asks; nothing changes.
    let asked = election.elect(&[OwnerId::new(1)], seconds(1), seconds(30))?;
    assert_eq!(asked, Leadership::Incumbent(lease));

    let renewed = election.heartbeat(lease.owner(), lease.token(), seconds(10), seconds(30))?;
    assert_eq!(renewed.expires_at(), seconds(40));
    let stranger = election.heartbeat(OwnerId::new(9), lease.token(), seconds(11), seconds(30));
    assert_eq!(stranger, Err(Refusal::Lease(LeaseRefusal::WrongOwner)));

    // Persist the checkpoint, then restore it under the same scope only.
    let saved = election.checkpoint();
    assert!(Election::restored(8, saved.clone(), ClockPolicy::Reject).is_none());
    let mut restored = Election::restored(7, saved, ClockPolicy::Reject).expect("the same scope");
    restored.resign(lease.owner(), lease.token(), seconds(12))?;
    let next = restored.elect(&candidates, seconds(12), seconds(30))?.lease();
    assert!(next.token().is_newer_than(lease.token()));
    Ok(())
}
```
