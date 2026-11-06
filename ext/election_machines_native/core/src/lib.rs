#![no_std]
#![doc = include_str!("../README.md")]
use core::time::Duration;
use lease_machines::{
    Checkpoint, Decision, FencingToken, Lease, LeaseAuthority, OwnerId, Rejection,
};

/// Outcome of nomination under one lease authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Leadership {
    /// A new leadership token was issued.
    Elected(Lease),
    /// The incumbent's unexpired lease remains authoritative.
    Incumbent(Lease),
    /// No candidates were supplied; no authority transition occurred.
    NoCandidates,
    /// The authority rejected the request.
    Rejected(Rejection),
}

/// Deterministic nomination plus a single resource's lease authority.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Election {
    authority: LeaseAuthority,
}

impl Election {
    /// Restore the newest committed authority under the same clock origin.
    #[must_use]
    pub const fn restore(checkpoint: Checkpoint) -> Self {
        Self {
            authority: LeaseAuthority::restore(checkpoint),
        }
    }

    /// Nominate the smallest participating identity, respecting a live incumbent.
    #[must_use]
    pub fn elect(&mut self, candidates: &[OwnerId], now: Duration, ttl: Duration) -> Leadership {
        if now < self.authority.checkpoint().observed() {
            return Leadership::Rejected(Rejection::ClockRegressed);
        }
        let Some(candidate) = candidates.iter().copied().min_by_key(|id| id.value()) else {
            return Leadership::NoCandidates;
        };
        match self.authority.acquire(candidate, now, ttl) {
            Decision::Granted(lease) => Leadership::Elected(lease),
            Decision::Rejected(Rejection::Occupied) => {
                // Occupied is returned only with a live incumbent.
                self.authority.checkpoint().lease().map_or(
                    Leadership::Rejected(Rejection::NoLease),
                    Leadership::Incumbent,
                )
            }
            Decision::Rejected(reason) => Leadership::Rejected(reason),
            _ => Leadership::Rejected(Rejection::NoLease),
        }
    }

    /// Renew leadership only for the matching owner and live fencing token.
    #[must_use]
    pub fn heartbeat(
        &mut self,
        owner: OwnerId,
        token: FencingToken,
        now: Duration,
        ttl: Duration,
    ) -> Decision {
        self.authority.renew(owner, token, now, ttl)
    }

    /// Relinquish leadership while retaining its fencing high-water mark.
    #[must_use]
    pub fn resign(&mut self, owner: OwnerId, token: FencingToken, now: Duration) -> Decision {
        self.authority.release(owner, token, now)
    }

    /// Capture state for the runtime's durable commit.
    #[must_use]
    pub const fn checkpoint(&self) -> Checkpoint {
        self.authority.checkpoint()
    }
}

#[cfg(test)]
mod tests;
