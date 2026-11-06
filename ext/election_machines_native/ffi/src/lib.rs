//! Thin Ruby translation for lease-backed nomination.
mod init;
mod values;
use election_machines::{Election, Leadership};
use lease_machines::{Checkpoint, Decision, FencingToken, Lease, OwnerId};
use magnus::{Error, RArray, Ruby, prelude::*, typed_data::Obj};
use std::{cell::RefCell, sync::Arc};
use values::{Id, Seconds};

type Outcome = (String, Option<u64>, Option<u64>, Option<f64>);

#[magnus::wrap(class = "ElectionMachines::Checkpoint", free_immediately, size)]
struct Saved {
    owner: Arc<()>,
    revision: u64,
    checkpoint: Checkpoint,
}
#[magnus::wrap(class = "ElectionMachines::Election", free_immediately, size)]
struct Wrapper {
    owner: Arc<()>,
    state: RefCell<(Election, u64)>,
}

fn lease_outcome(status: &str, lease: Lease) -> Outcome {
    (
        status.into(),
        Some(lease.owner().value()),
        Some(lease.token().value()),
        Some(lease.expires_at().as_secs_f64()),
    )
}
fn decision(result: Decision) -> Outcome {
    match result {
        Decision::Granted(lease) => lease_outcome("granted", lease),
        Decision::Renewed(lease) => lease_outcome("renewed", lease),
        Decision::Released => ("released".into(), None, None, None),
        Decision::Rejected(reason) => (format!("{reason:?}"), None, None, None),
        _ => ("unknown".into(), None, None, None),
    }
}
impl Wrapper {
    fn new() -> Self {
        Self {
            owner: Arc::new(()),
            state: RefCell::new((Election::default(), 0)),
        }
    }
    fn next_revision(&self, ruby: &Ruby) -> Result<u64, Error> {
        let revision = self.state.borrow().1.checked_add(1);
        revision.ok_or_else(|| Error::new(ruby.exception_range_error(), "revision exhausted"))
    }
    /// Runs `step` against the election under the next revision, which stales
    /// every checkpoint taken before it.
    fn advance<T>(&self, ruby: &Ruby, step: impl FnOnce(&mut Election) -> T) -> Result<T, Error> {
        let revision = self.next_revision(ruby)?;
        let mut state = self.state.borrow_mut();
        state.1 = revision;
        Ok(step(&mut state.0))
    }
    fn elect(
        ruby: &Ruby,
        rb_self: Obj<Self>,
        candidates: RArray,
        now: Seconds,
        ttl: Seconds,
    ) -> Result<Outcome, Error> {
        rb_self.check_frozen()?;
        values::interval(ruby, now, ttl)?;
        let count = candidates.len();
        if count > 64 {
            return Err(Error::new(
                ruby.exception_arg_error(),
                "at most 64 candidates",
            ));
        }
        let mut owners = [OwnerId::new(0); 64];
        for (index, owner) in owners[..count].iter_mut().enumerate() {
            *owner = OwnerId::new(candidates.entry::<Id>(index.cast_signed())?.0);
        }
        let result = rb_self.advance(ruby, |election| {
            election.elect(&owners[..count], now.0, ttl.0)
        })?;
        Ok(match result {
            Leadership::Elected(lease) => lease_outcome("elected", lease),
            Leadership::Incumbent(lease) => lease_outcome("incumbent", lease),
            Leadership::NoCandidates => ("no_candidates".into(), None, None, None),
            Leadership::Rejected(reason) => (format!("{reason:?}"), None, None, None),
            _ => ("unknown".into(), None, None, None),
        })
    }
    fn heartbeat(
        ruby: &Ruby,
        rb_self: Obj<Self>,
        owner: Id,
        token: Id,
        now: Seconds,
        ttl: Seconds,
    ) -> Result<Outcome, Error> {
        rb_self.check_frozen()?;
        values::interval(ruby, now, ttl)?;
        let token = values::token(ruby, token)?;
        let result = rb_self.advance(ruby, |election| {
            election.heartbeat(OwnerId::new(owner.0), token, now.0, ttl.0)
        })?;
        Ok(decision(result))
    }
    fn resign(
        ruby: &Ruby,
        rb_self: Obj<Self>,
        owner: Id,
        token: Id,
        now: Seconds,
    ) -> Result<Outcome, Error> {
        rb_self.check_frozen()?;
        let token: FencingToken = values::token(ruby, token)?;
        let result = rb_self.advance(ruby, |election| {
            election.resign(OwnerId::new(owner.0), token, now.0)
        })?;
        Ok(decision(result))
    }
    fn checkpoint(&self) -> Saved {
        let (checkpoint, revision) = {
            let state = self.state.borrow();
            (state.0.checkpoint(), state.1)
        };
        Saved {
            owner: Arc::clone(&self.owner),
            revision,
            checkpoint,
        }
    }
    fn restore(ruby: &Ruby, rb_self: Obj<Self>, saved: &Saved) -> Result<bool, Error> {
        rb_self.check_frozen()?;
        if !Arc::ptr_eq(&rb_self.owner, &saved.owner) || rb_self.state.borrow().1 != saved.revision
        {
            return Err(Error::new(
                ruby.exception_arg_error(),
                "cross-instance or stale checkpoint",
            ));
        }
        let revision = rb_self.next_revision(ruby)?;
        *rb_self.state.borrow_mut() = (Election::restore(saved.checkpoint), revision);
        Ok(true)
    }
    fn lease(&self) -> Option<(u64, u64, f64)> {
        self.state.borrow().0.checkpoint().lease().map(|lease| {
            (
                lease.owner().value(),
                lease.token().value(),
                lease.expires_at().as_secs_f64(),
            )
        })
    }
}
