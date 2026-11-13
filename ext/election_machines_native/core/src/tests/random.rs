//! Random calls against the election and the naive model, side by side.
use std::vec::Vec;

use super::model::{Grant, Model};
use super::{ANSWERED, LAST, at, token};
use crate::{Checkpoint, ClockPolicy, Duration, Election, Leadership, Lease, OwnerId, Refusal};

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }
}

fn grant(lease: Lease) -> Grant {
    let sequence = lease.token().raw() & LAST;
    (lease.owner().value(), sequence, lease.expires_at())
}

/// The whole election restated from its readers, to compare with the model.
fn view(election: &Election) -> Model {
    let saved = election.checkpoint();
    Model {
        scope: election.scope(),
        issued: election.issued(),
        grant: election.lease().map(grant),
        seen: saved.observed(),
        clock: election.clock_policy(),
    }
}

struct Run {
    election: Election,
    model: Model,
    rng: Rng,
    clock: u64,
    refusals: Vec<Refusal>,
    incumbents: usize,
}

impl Run {
    fn new(seed: u64, scope: u16, issued: u64, clock: ClockPolicy) -> Self {
        let saved = Checkpoint::from_parts(scope, issued, None, at(0)).unwrap();
        let election = Election::restored(scope, saved, clock).unwrap();
        let model = Model {
            scope,
            issued,
            grant: None,
            seen: at(0),
            clock,
        };
        Self {
            election,
            model,
            rng: Rng(seed),
            clock: 0,
            refusals: Vec::new(),
            incumbents: 0,
        }
    }

    /// Mostly forward, sometimes back, in nanoseconds.
    fn now(&mut self) -> Duration {
        if self.rng.below(8) == 0 {
            return Duration::from_nanos(self.rng.below(self.clock + 1));
        }
        self.clock += self.rng.below(6);
        Duration::from_nanos(self.clock)
    }

    fn ttl(&mut self) -> Duration {
        match self.rng.below(12) {
            0 => Duration::ZERO,
            1 => Duration::MAX,
            _ => Duration::from_nanos(1 + self.rng.below(9)),
        }
    }

    /// The token in force, an older one, an unissued one, or another scope's.
    fn token(&mut self) -> (u16, u64) {
        let issued = self.model.issued;
        let current = self
            .model
            .grant
            .map_or_else(|| issued.max(1), |grant| grant.1);
        match self.rng.below(8) {
            0 => (self.model.scope, 1 + self.rng.below(issued.max(1))),
            1 => (self.model.scope, (issued + 1).min(LAST)),
            2 => (self.model.scope.wrapping_add(1), current),
            _ => (self.model.scope, current),
        }
    }

    /// Compare the answers, then the whole state; a refusal and an incumbent
    /// answer must also leave the election equal to its clone.
    fn check<T: PartialEq + core::fmt::Debug>(
        &mut self,
        before: &Election,
        got: &Result<T, Refusal>,
        want: &Result<T, Refusal>,
        unchanged: bool,
    ) {
        assert_eq!(got, want);
        assert_eq!(view(&self.election), self.model);
        if let Err(refusal) = *got {
            assert_eq!(self.election, *before, "{refusal:?} changed the election");
            self.refusals.push(refusal);
        } else if unchanged {
            assert_eq!(self.election, *before, "an incumbent answer changed it");
            self.incumbents += 1;
        }
    }

    fn step(&mut self) {
        let before = self.election.clone();
        let owner = self.rng.below(4);
        match self.rng.below(10) {
            0..=3 => {
                let count = self.rng.below(4);
                let ids: Vec<u64> = (0..count).map(|_| self.rng.below(4)).collect();
                let owners: Vec<OwnerId> = ids.iter().copied().map(OwnerId::new).collect();
                let (now, ttl) = (self.now(), self.ttl());
                let got = self.election.elect(&owners, now, ttl);
                let want = self.model.elect(&ids, now, ttl);
                let unchanged = matches!(got, Ok(Leadership::Incumbent(_)));
                let got = got.map(|led| (led.is_incumbent(), grant(led.lease())));
                self.check(&before, &got, &want, unchanged);
            }
            4..=7 => {
                let (held, now, ttl) = (self.token(), self.now(), self.ttl());
                let presented = token(held.0, held.1);
                let got = self
                    .election
                    .heartbeat(OwnerId::new(owner), presented, now, ttl);
                let want = self.model.heartbeat(owner, held, now, ttl);
                self.check(&before, &got.map(grant), &want, false);
            }
            _ => {
                let (held, now) = (self.token(), self.now());
                let presented = token(held.0, held.1);
                let got = self.election.resign(OwnerId::new(owner), presented, now);
                let want = self.model.resign(owner, held, now);
                self.check(&before, &got.map(grant), &want, false);
            }
        }
    }
}

#[test]
fn the_election_agrees_with_the_naive_model() {
    let mut seen = Vec::new();
    let mut incumbents = 0;
    for seed in 1..=40_u64 {
        let clock = if seed % 2 == 0 {
            ClockPolicy::Reject
        } else {
            ClockPolicy::Clamp
        };
        // Every fourth run starts a few tokens short of the end of the space.
        let issued = if seed % 4 == 0 { LAST - 3 } else { seed % 3 };
        let scope = u16::try_from(seed % 3).unwrap() * 0x7fff;
        let mut run = Run::new(
            seed.wrapping_mul(0x9e37_79b9_7f4a_7c15),
            scope,
            issued,
            clock,
        );
        for _ in 0..3_000 {
            run.step();
        }
        incumbents += run.incumbents;
        seen.extend(run.refusals);
    }
    assert!(incumbents > 0);
    // Every refusal an election call can answer was reached, and no other.
    for refusal in ANSWERED {
        assert!(seen.contains(&refusal), "{refusal:?} never answered");
    }
    for refusal in seen {
        assert!(ANSWERED.contains(&refusal), "{refusal:?} answered");
    }
}
