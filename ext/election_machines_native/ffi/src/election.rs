//! `ElectionMachines::Election`.
use std::cell::RefCell;

use election_machines::{Leadership, Lease, OwnerId, Refusal};
use magnus::{Error, IntoValue, RArray, Ruby, Value, prelude::*, typed_data::Obj};

use crate::classes::{classes, keep};
use crate::names::{self, Policy};
use crate::values::{CheckpointValue, TokenValue, lease_object};
use crate::{Nanos, Number, Scope, row};

type Core = election_machines::Election;

#[magnus::wrap(class = "ElectionMachines::Election", free_immediately, size)]
pub struct Election(RefCell<Core>);

impl Election {
    pub const fn with_scope(scope: Scope) -> Self {
        Self(RefCell::new(Core::with_scope(scope.0)))
    }

    pub fn restored(
        ruby: &Ruby,
        scope: Scope,
        saved: CheckpointValue,
        clock: Policy,
    ) -> Result<Self, Error> {
        Core::restored(scope.0, saved.0, clock.0)
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

    /// An `Elected` or `Incumbent` when honoured, or the refusal symbol.
    /// Every candidate is converted before the core is called. The answer is
    /// built while the decision is held on a copy, kept only once it exists
    /// (clone-then-keep).
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
        let classes = classes(ruby)?;
        let decide = |core: &mut Core| core.elect(&owners, now.0, ttl.0);
        keep(ruby, this, &this.0, decide, |outcome| {
            let (class, lease) = match outcome {
                Ok(Leadership::Elected(lease)) => (classes.elected, lease),
                Ok(Leadership::Incumbent(lease)) => (classes.incumbent, lease),
                Err(refusal) => return Ok(Self::refused(ruby, refusal)),
            };
            ruby.get_inner(class)
                .funcall("new", (lease_object(ruby, lease)?,))
        })
    }

    /// A `Lease` when honoured, or the refusal symbol. Everything that can
    /// raise happens before the core is called; the `Lease` is built while
    /// the decision is held on a copy (clone-then-keep).
    fn decide(
        ruby: &Ruby,
        this: Obj<Self>,
        call: impl FnOnce(&mut Core) -> Result<Lease, Refusal>,
    ) -> Result<Value, Error> {
        keep(ruby, this, &this.0, call, |decision| match decision {
            Ok(lease) => lease_object(ruby, lease),
            Err(refusal) => Ok(Self::refused(ruby, refusal)),
        })
    }

    pub fn heartbeat(
        ruby: &Ruby,
        this: Obj<Self>,
        owner: Number,
        token: TokenValue,
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
        token: TokenValue,
        now: Nanos,
    ) -> Result<Value, Error> {
        Self::decide(ruby, this, |core| {
            core.resign(OwnerId::new(owner.0), token.0, now.0)
        })
    }
}
