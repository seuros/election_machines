//! A deliberately naive restatement of the election rules, in plain numbers.
use crate::{ClockPolicy, Duration, LeaseRefusal, Refusal};

use super::LAST;

/// A grant as `(owner, sequence, expires_at)`.
pub type Grant = (u64, u64, Duration);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Model {
    pub scope: u16,
    pub issued: u64,
    pub grant: Option<Grant>,
    pub seen: Duration,
    pub clock: ClockPolicy,
}

impl Model {
    fn at(&self, now: Duration) -> Result<Duration, Refusal> {
        if now >= self.seen {
            return Ok(now);
        }
        match self.clock {
            ClockPolicy::Reject => Err(LeaseRefusal::ClockRegressed.into()),
            ClockPolicy::Clamp => Ok(self.seen),
        }
    }

    fn deadline(now: Duration, ttl: Duration) -> Result<Duration, Refusal> {
        let deadline = now.checked_add(ttl).filter(|_| !ttl.is_zero());
        deadline.ok_or(Refusal::Lease(LeaseRefusal::InvalidTtl))
    }

    /// `Ok((incumbent, grant))`.
    pub fn elect(
        &mut self,
        candidates: &[u64],
        now: Duration,
        ttl: Duration,
    ) -> Result<(bool, Grant), Refusal> {
        let nominee = *candidates.iter().min().ok_or(Refusal::NoCandidates)?;
        let now = self.at(now)?;
        let deadline = Self::deadline(now, ttl)?;
        if let Some(grant) = self.grant
            && now < grant.2
        {
            return Ok((true, grant));
        }
        if self.issued == LAST {
            return Err(LeaseRefusal::Exhausted.into());
        }
        self.issued += 1;
        self.seen = now;
        let grant = (nominee, self.issued, deadline);
        self.grant = Some(grant);
        Ok((false, grant))
    }

    /// The instant the call is evaluated at, when `owner` leads under the
    /// token `(scope, sequence)`.
    fn leads(
        &self,
        owner: u64,
        scope: u16,
        sequence: u64,
        now: Duration,
    ) -> Result<(Duration, Grant), Refusal> {
        if scope != self.scope || sequence > self.issued {
            return Err(LeaseRefusal::Unknown.into());
        }
        let now = self.at(now)?;
        let grant = self.grant.ok_or(Refusal::Lease(LeaseRefusal::NoLease))?;
        if grant.0 != owner {
            Err(LeaseRefusal::WrongOwner.into())
        } else if grant.1 != sequence {
            Err(LeaseRefusal::StaleToken.into())
        } else if now >= grant.2 {
            Err(LeaseRefusal::Expired.into())
        } else {
            Ok((now, grant))
        }
    }

    pub fn heartbeat(
        &mut self,
        owner: u64,
        token: (u16, u64),
        now: Duration,
        ttl: Duration,
    ) -> Result<Grant, Refusal> {
        let (now, grant) = self.leads(owner, token.0, token.1, now)?;
        let deadline = Self::deadline(now, ttl)?;
        let grant = (grant.0, grant.1, grant.2.max(deadline));
        self.grant = Some(grant);
        self.seen = now;
        Ok(grant)
    }

    pub fn resign(
        &mut self,
        owner: u64,
        token: (u16, u64),
        now: Duration,
    ) -> Result<Grant, Refusal> {
        let (now, grant) = self.leads(owner, token.0, token.1, now)?;
        self.grant = None;
        self.seen = now;
        Ok(grant)
    }
}
