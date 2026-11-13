//! `ElectionMachines::Election`.
use std::cell::RefCell;

use election_machines::{Leadership, Lease, OwnerId, Refusal};
use magnus::{Error, IntoValue, RArray, Ruby, Value, prelude::*, typed_data::Obj};

use crate::names::{self, Policy};
use crate::{Nanos, Number, Saved, Scope, Token, checkpoint, row};

type Core = election_machines::Election;

#[magnus::wrap(class = "ElectionMachines::Election", free_immediately, size)]
pub struct Election(RefCell<Core>);

impl Election {
    pub const fn with_scope(scope: Scope) -> Self {
        Self(RefCell::new(Core::with_scope(scope.0)))
    }

    pub fn restored(ruby: &Ruby, scope: Scope, saved: Saved, clock: Policy) -> Result<Self, Error> {
        let committed = checkpoint(ruby, saved)?;
        Core::restored(scope.0, committed, clock.0)
            .map(|core| Self(RefCell::new(core)))
            .ok_or_else(|| {
                Error::new(
                    ruby.exception_range_error(),
                    "the checkpoint is of another scope",
                )
            })
    }

    pub fn with_clock_policy(this: Obj<Self>, clock: Policy) -> Result<(), Error> {
        this.check_frozen()?;
        let mut core = this.0.borrow_mut();
        *core = core.clone().with_clock_policy(clock.0);
        Ok(())
    }

    pub fn clock_policy(ruby: &Ruby, this: &Self) -> magnus::Symbol {
        ruby.to_symbol(names::clock_policy(this.0.borrow().clock_policy()))
    }

    pub fn scope(&self) -> u16 {
        self.0.borrow().scope()
    }

    pub fn issued(&self) -> u64 {
        self.0.borrow().issued()
    }

    pub fn lease(ruby: &Ruby, this: &Self) -> Option<Value> {
        this.0.borrow().lease().map(|held| row(ruby, held))
    }

    pub fn checkpoint(ruby: &Ruby, this: &Self) -> Value {
        let saved = this.0.borrow().checkpoint();
        (
            saved.scope(),
            saved.issued(),
            saved.lease().map(|held| row(ruby, held)),
            saved.observed().as_nanos(),
        )
            .into_value_with(ruby)
    }

    fn refused(ruby: &Ruby, refusal: Refusal) -> Value {
        ruby.to_symbol(names::refusal(refusal)).as_value()
    }

    /// `[incumbent, lease row]` when honoured, or the refusal symbol.
    pub fn elect(
        ruby: &Ruby,
        this: Obj<Self>,
        candidates: RArray,
        now: Nanos,
        ttl: Nanos,
    ) -> Result<Value, Error> {
        this.check_frozen()?;
        let mut owners = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            owners.push(OwnerId::new(Number::try_convert(candidate)?.0));
        }
        let outcome = this.0.borrow_mut().elect(&owners, now.0, ttl.0);
        Ok(match outcome {
            Ok(Leadership::Elected(lease)) => (false, row(ruby, lease)).into_value_with(ruby),
            Ok(Leadership::Incumbent(lease)) => (true, row(ruby, lease)).into_value_with(ruby),
            Err(refusal) => Self::refused(ruby, refusal),
        })
    }

    /// The lease row when honoured, or the refusal symbol.
    fn decide(
        ruby: &Ruby,
        this: Obj<Self>,
        call: impl FnOnce(&mut Core) -> Result<Lease, Refusal>,
    ) -> Result<Value, Error> {
        this.check_frozen()?;
        let decision = call(&mut this.0.borrow_mut());
        Ok(match decision {
            Ok(lease) => row(ruby, lease),
            Err(refusal) => Self::refused(ruby, refusal),
        })
    }

    pub fn heartbeat(
        ruby: &Ruby,
        this: Obj<Self>,
        owner: Number,
        token: Token,
        now: Nanos,
        ttl: Nanos,
    ) -> Result<Value, Error> {
        Self::decide(ruby, this, |core| {
            core.heartbeat(OwnerId::new(owner.0), token.0, now.0, ttl.0)
        })
    }

    pub fn resign(
        ruby: &Ruby,
        this: Obj<Self>,
        owner: Number,
        token: Token,
        now: Nanos,
    ) -> Result<Value, Error> {
        Self::decide(ruby, this, |core| {
            core.resign(OwnerId::new(owner.0), token.0, now.0)
        })
    }
}
