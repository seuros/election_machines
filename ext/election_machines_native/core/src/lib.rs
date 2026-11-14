#![no_std]
#![doc = include_str!("../README.md")]

mod refusal;

use core::time::Duration;

use lease_machines::LeaseAuthority;
pub use lease_machines::Refusal as LeaseRefusal;
pub use lease_machines::{Checkpoint, ClockPolicy, FencingToken, Lease, OwnerId};
pub use refusal::Refusal;

/// Honoured outcome of [`Election::elect`]: who leads, and whether this call
/// made them leader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Leadership {
    /// The smallest candidate was elected under a new, greater token.
    Elected(Lease),
    /// A leader's unexpired lease is still in force, whoever the candidates
    /// were. Nothing changed, not even the observed clock.
    Incumbent(Lease),
}

impl Leadership {
    /// The leadership in force after the call.
    #[must_use]
    pub const fn lease(self) -> Lease {
        match self {
            Self::Elected(lease) | Self::Incumbent(lease) => lease,
        }
    }

    /// Whether the call found a live leader and changed nothing.
    #[must_use]
    pub const fn is_incumbent(self) -> bool {
        match self {
            Self::Elected(_) => false,
            Self::Incumbent(_) => true,
        }
    }
}

/// Deterministic nomination plus a single resource's lease authority.
///
/// The election issues no ids of its own: leadership tokens come from the
/// embedded authority's sequence, and carry its scope. Cloning an election
/// does not create a second one: two instances serving one scope issue the
/// same tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Election {
    authority: LeaseAuthority,
}

impl Default for Election {
    fn default() -> Self {
        Self::new()
    }
}

impl Election {
    /// Start a **fresh** token namespace in scope zero. Never use this to
    /// recover an old one: it issues the same tokens again.
    #[must_use]
    pub const fn new() -> Self {
        Self::with_scope(0)
    }

    /// Start a **fresh** token namespace whose tokens carry `scope`.
    ///
    /// Give every election whose tokens could meet its own scope; a token of
    /// another scope is then [`LeaseRefusal::Unknown`]. Keeping scopes distinct
    /// is the caller's job.
    #[must_use]
    pub const fn with_scope(scope: u16) -> Self {
        Self {
            authority: LeaseAuthority::with_scope(scope),
        }
    }

    /// Restore the newest committed checkpoint of `scope`, under the same
    /// clock origin, with the clock policy stated again.
    ///
    /// The scope is the caller's, never read out of the checkpoint: `None`
    /// when the checkpoint is of another scope. The caller must prevent
    /// simultaneous serving by the previous election.
    #[must_use]
    pub const fn restored(scope: u16, checkpoint: Checkpoint, clock: ClockPolicy) -> Option<Self> {
        match LeaseAuthority::restored(scope, checkpoint, clock) {
            Some(authority) => Some(Self { authority }),
            None => None,
        }
    }

    /// Choose how earlier-than-observed clocks are handled; `Reject` by default.
    #[must_use]
    pub const fn with_clock_policy(self, clock: ClockPolicy) -> Self {
        Self {
            authority: self.authority.with_clock_policy(clock),
        }
    }

    /// Policy applied to regressing observations.
    #[must_use]
    pub const fn clock_policy(&self) -> ClockPolicy {
        self.authority.clock_policy()
    }

    /// The scope carried by every token this election issues.
    #[must_use]
    pub const fn scope(&self) -> u16 {
        self.authority.scope()
    }

    /// How many tokens have been issued; also the sequence number of the last.
    #[must_use]
    pub const fn issued(&self) -> u64 {
        self.authority.issued()
    }

    /// The last leadership granted; it may already be expired, and it is
    /// `None` after a resignation.
    #[must_use]
    pub const fn lease(&self) -> Option<Lease> {
        self.authority.checkpoint().lease()
    }

    /// Capture state for the runtime's durable commit. Persist it before
    /// exposing the decision that produced it.
    #[must_use]
    pub const fn checkpoint(&self) -> Checkpoint {
        self.authority.checkpoint()
    }

    /// Nominate the smallest candidate, respecting a live incumbent.
    ///
    /// A leader whose lease has not expired stays leader even when it is not
    /// among `candidates`, and even when it is the smallest of them: the call
    /// answers [`Leadership::Incumbent`] with the lease in force and changes
    /// nothing. Otherwise the smallest candidate is elected under a new token.
    ///
    /// # Errors
    /// `NoCandidates` (checked first, whatever the clock), or the authority's
    /// refusal of the grant: `Lease(ClockRegressed)`, `Lease(InvalidTtl)` or
    /// `Lease(Exhausted)`. No other lease refusal arrives: `Occupied` and
    /// `Held` are the live incumbent, which is honoured, and the rest belong
    /// to calls `elect` does not make.
    pub fn elect(
        &mut self,
        candidates: &[OwnerId],
        now: Duration,
        ttl: Duration,
    ) -> Result<Leadership, Refusal> {
        let nominee = candidates.iter().copied().min();
        let nominee = nominee.ok_or(Refusal::NoCandidates)?;
        match self.authority.acquire(nominee, now, ttl) {
            Ok(lease) => Ok(Leadership::Elected(lease)),
            // Both name a grant that is unexpired at the instant the request
            // was evaluated at; a refused acquire recorded nothing, so the
            // incumbent answer records nothing either.
            Err(refusal @ (LeaseRefusal::Occupied | LeaseRefusal::Held)) => {
                if let Some(lease) = self.lease() {
                    return Ok(Leadership::Incumbent(lease));
                }
                debug_assert!(
                    false,
                    "the authority answers Occupied and Held only about a grant it holds"
                );
                // Nothing changed: carry the authority's refusal as it came.
                Err(Refusal::Lease(refusal))
            }
            // Listed in full, so a new lease refusal is decided here: is it
            // an incumbent to honour, or a nomination refused?
            Err(
                refusal @ (LeaseRefusal::ClockRegressed
                | LeaseRefusal::InvalidTtl
                | LeaseRefusal::Exhausted
                | LeaseRefusal::NoLease
                | LeaseRefusal::WrongOwner
                | LeaseRefusal::StaleToken
                | LeaseRefusal::Unknown
                | LeaseRefusal::Expired
                | LeaseRefusal::Fenced
                | LeaseRefusal::OutOfPhase),
            ) => Err(Refusal::Lease(refusal)),
        }
    }

    /// Renew leadership only for the matching owner and live fencing token,
    /// retaining the token.
    ///
    /// A shorter requested TTL does not shorten the previous expiration, so a
    /// repeated or reordered heartbeat is honoured and never loses time.
    ///
    /// # Errors
    /// The authority's refusal of the renewal, carried whole: `Lease(Unknown)`,
    /// `Lease(ClockRegressed)`, `Lease(NoLease)`, `Lease(WrongOwner)`,
    /// `Lease(StaleToken)`, `Lease(Expired)` or `Lease(InvalidTtl)`.
    pub fn heartbeat(
        &mut self,
        owner: OwnerId,
        token: FencingToken,
        now: Duration,
        ttl: Duration,
    ) -> Result<Lease, Refusal> {
        let renewed = self.authority.renew(owner, token, now, ttl);
        renewed.map_err(Refusal::Lease)
    }

    /// Relinquish unexpired leadership, answering the lease that was given
    /// up. The token count is retained: the next token is still greater.
    ///
    /// A repeat is `Lease(NoLease)`, not an honoured duplicate: the election keeps
    /// no record of who resigned what, so it cannot prove the caller's claim.
    ///
    /// # Errors
    /// The authority's refusal of the release, carried whole: `Lease(Unknown)`,
    /// `Lease(ClockRegressed)`, `Lease(NoLease)`, `Lease(WrongOwner)`,
    /// `Lease(StaleToken)` or `Lease(Expired)`.
    pub fn resign(
        &mut self,
        owner: OwnerId,
        token: FencingToken,
        now: Duration,
    ) -> Result<Lease, Refusal> {
        let released = self.authority.release(owner, token, now);
        released.map_err(Refusal::Lease)
    }
}

#[cfg(test)]
extern crate std;
#[cfg(test)]
mod tests;
