//! `ElectionMachines::Election`.
use std::cell::RefCell;

use election_machines::{Leadership, Lease, OwnerId, Refusal};
use magnus::{Error, IntoValue, Ruby, Value, prelude::*, typed_data::Obj};
use matryona_machines::ruby::{
    SymbolNames, Symbols, exact_array, instant, integer, keep, nanoseconds,
};

use crate::classes::classes;
use crate::values::{self, lease_object};
use crate::{names, row};

fn owner(value: Value) -> Result<OwnerId, Error> {
    integer(value, "owner").map(OwnerId::new)
}

type Core = election_machines::Election;

#[magnus::wrap(class = "ElectionMachines::Election", free_immediately, size)]
pub struct Election(RefCell<Core>);

impl Election {
    pub fn with_scope(scope: Value) -> Result<Self, Error> {
        Ok(Self(RefCell::new(Core::with_scope(integer(
            scope, "scope",
        )?))))
    }

    pub fn restored(ruby: &Ruby, scope: Value, saved: Value, clock: Value) -> Result<Self, Error> {
        let (scope, saved) = (integer(scope, "scope")?, values::checkpoint(ruby, saved)?);
        Core::restored(scope, saved, names::ClockPolicies::parse(ruby, clock)?)
            .map(|core| Self(RefCell::new(core)))
            .ok_or_else(|| {
                Error::new(
                    ruby.exception_range_error(),
                    "the checkpoint is of another scope",
                )
            })
    }

    pub fn with_clock_policy(ruby: &Ruby, this: Obj<Self>, clock: Value) -> Result<(), Error> {
        this.check_frozen()?;
        let clock = names::ClockPolicies::parse(ruby, clock)?;
        let mut core = this.0.borrow_mut();
        *core = core.clone().with_clock_policy(clock);
        Ok(())
    }

    pub fn clock_policy(ruby: &Ruby, this: &Self) -> magnus::Symbol {
        names::ClockPolicies::symbol(ruby, this.0.borrow().clock_policy())
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
            nanoseconds(ruby, saved.observed()),
        )
            .into_value_with(ruby)
    }

    fn refused(ruby: &Ruby, refusal: Refusal) -> Value {
        names::Refusals::symbol(ruby, &refusal).as_value()
    }

    /// An `Elected` or `Incumbent` when honoured, or the refusal symbol.
    /// Every candidate is converted before the core is called. The answer is
    /// built while the decision is held on a copy, kept only once it exists
    /// (clone-then-keep).
    pub fn elect(
        ruby: &Ruby,
        this: Obj<Self>,
        candidates: Value,
        now: Value,
        ttl: Value,
    ) -> Result<Value, Error> {
        this.check_frozen()?;
        let message = "candidates must be an Array of Integers";
        let candidates = exact_array(ruby, candidates, message)?;
        let mut owners = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            owners.push(OwnerId::new(integer(candidate, "candidate")?));
        }
        let (now, ttl) = (instant(now, "now")?, instant(ttl, "ttl")?);
        let classes = classes(ruby)?;
        let decide = |core: &mut Core| core.elect(&owners, now, ttl);
        keep(ruby, this, &this.0, decide, |outcome| {
            let (class, lease) = match outcome {
                Ok(Leadership::Elected(lease)) => (classes.elected, lease),
                Ok(Leadership::Incumbent(lease)) => (classes.incumbent, lease),
                Err(refusal) => return Ok(Self::refused(ruby, refusal)),
            };
            class
                .get(ruby)
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
        owner: Value,
        token: Value,
        now: Value,
        ttl: Value,
    ) -> Result<Value, Error> {
        this.check_frozen()?;
        let (owner, token) = (self::owner(owner)?, values::token(ruby, token)?);
        let (now, ttl) = (instant(now, "now")?, instant(ttl, "ttl")?);
        Self::decide(ruby, this, |core| core.heartbeat(owner, token, now, ttl))
    }

    pub fn resign(
        ruby: &Ruby,
        this: Obj<Self>,
        owner: Value,
        token: Value,
        now: Value,
    ) -> Result<Value, Error> {
        this.check_frozen()?;
        let (owner, token) = (self::owner(owner)?, values::token(ruby, token)?);
        let now = instant(now, "now")?;
        Self::decide(ruby, this, |core| core.resign(owner, token, now))
    }
}
