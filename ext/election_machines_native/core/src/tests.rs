//! Contract tests, and a randomized comparison against a naive model.
#![allow(clippy::unwrap_used)] // Failed test assumptions should panic.
mod contract;
mod model;
mod random;
mod restart;

use std::vec::Vec;

use crate::{Duration, Election, FencingToken, Leadership, Lease, LeaseRefusal, OwnerId, Refusal};

/// Every refusal an election call can answer; the randomized run reaches
/// each of them and nothing else. `Occupied` and `Held` are honoured as an
/// incumbent, and `Fenced` and `OutOfPhase` are the lease holder's.
const ANSWERED: [Refusal; 9] = [
    Refusal::NoCandidates,
    Refusal::Lease(LeaseRefusal::NoLease),
    Refusal::Lease(LeaseRefusal::WrongOwner),
    Refusal::Lease(LeaseRefusal::StaleToken),
    Refusal::Lease(LeaseRefusal::Unknown),
    Refusal::Lease(LeaseRefusal::Expired),
    Refusal::Lease(LeaseRefusal::ClockRegressed),
    Refusal::Lease(LeaseRefusal::InvalidTtl),
    Refusal::Lease(LeaseRefusal::Exhausted),
];

/// Sequence numbers one scope holds: 2^48 - 1.
const LAST: u64 = (1 << 48) - 1;

fn at(seconds: u64) -> Duration {
    Duration::from_secs(seconds)
}

fn owners(ids: &[u64]) -> Vec<OwnerId> {
    ids.iter().copied().map(OwnerId::new).collect()
}

/// The token `scope` issues at `sequence`, whoever asks.
fn token(scope: u16, sequence: u64) -> FencingToken {
    FencingToken::from_raw((u64::from(scope) << 48) | sequence).unwrap()
}

fn elected(election: &mut Election, ids: &[u64], now: u64, ttl: u64) -> Lease {
    match election.elect(&owners(ids), at(now), at(ttl)) {
        Ok(Leadership::Elected(lease)) => lease,
        other => panic!("expected an election, got {other:?}"),
    }
}

/// Run `call`, which must be refused with `refusal` and change nothing.
fn refused<T: core::fmt::Debug + PartialEq>(
    election: &mut Election,
    refusal: impl Into<Refusal>,
    call: impl FnOnce(&mut Election) -> Result<T, Refusal>,
) {
    let refusal = refusal.into();
    let before = election.clone();
    assert_eq!(call(election), Err(refusal));
    assert_eq!(*election, before, "{refusal:?} changed the election");
}
