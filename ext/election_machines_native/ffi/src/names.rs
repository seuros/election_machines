//! Symbol names for the core's enums. Every match is exhaustive: a new
//! variant fails to compile here until it has a name.
use election_machines::{ClockPolicy, LeaseRefusal, Refusal};
use magnus::{Error, Ruby, Symbol, TryConvert, Value};

/// Exactly the refusals an election call can answer: its own, and the lease
/// refusals `elect`, `heartbeat` and `resign` can carry.
pub const REFUSALS: [&str; 9] = [
    "no_candidates",
    "no_lease",
    "wrong_owner",
    "stale_token",
    "unknown",
    "expired",
    "clock_regressed",
    "invalid_ttl",
    "exhausted",
];

pub const CLOCK_POLICIES: [&str; 2] = ["reject", "clamp"];

/// A carried lease refusal answers the lease core's own symbol.
pub const fn refusal(refusal: Refusal) -> &'static str {
    match refusal {
        Refusal::NoCandidates => "no_candidates",
        Refusal::Lease(LeaseRefusal::Occupied) => "occupied",
        Refusal::Lease(LeaseRefusal::Held) => "held",
        Refusal::Lease(LeaseRefusal::NoLease) => "no_lease",
        Refusal::Lease(LeaseRefusal::WrongOwner) => "wrong_owner",
        Refusal::Lease(LeaseRefusal::StaleToken) => "stale_token",
        Refusal::Lease(LeaseRefusal::Unknown) => "unknown",
        Refusal::Lease(LeaseRefusal::Expired) => "expired",
        Refusal::Lease(LeaseRefusal::ClockRegressed) => "clock_regressed",
        Refusal::Lease(LeaseRefusal::InvalidTtl) => "invalid_ttl",
        Refusal::Lease(LeaseRefusal::Exhausted) => "exhausted",
        Refusal::Lease(LeaseRefusal::Fenced) => "fenced",
        Refusal::Lease(LeaseRefusal::OutOfPhase) => "out_of_phase",
    }
}

pub const fn clock_policy(policy: ClockPolicy) -> &'static str {
    match policy {
        ClockPolicy::Reject => "reject",
        ClockPolicy::Clamp => "clamp",
    }
}

/// A clock policy named by its symbol.
#[derive(Clone, Copy)]
pub struct Policy(pub ClockPolicy);

impl TryConvert for Policy {
    fn try_convert(value: Value) -> Result<Self, Error> {
        let ruby = Ruby::get_with(value);
        let symbol = Symbol::from_value(value).ok_or_else(|| {
            Error::new(ruby.exception_type_error(), "clock policy must be a Symbol")
        })?;
        let name = symbol.name()?;
        if name == clock_policy(ClockPolicy::Reject) {
            Ok(Self(ClockPolicy::Reject))
        } else if name == clock_policy(ClockPolicy::Clamp) {
            Ok(Self(ClockPolicy::Clamp))
        } else {
            Err(Error::new(
                ruby.exception_arg_error(),
                "clock policy must be :reject or :clamp",
            ))
        }
    }
}
