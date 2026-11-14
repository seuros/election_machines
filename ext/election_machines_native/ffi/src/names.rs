//! Symbol names for the core's enums. Every match is exhaustive: a new
//! variant fails to compile here until it has a name.
use election_machines::{ClockPolicy, LeaseRefusal, Refusal};
use magnus::{Ruby, Symbol};
use matryona_machines::ruby::{Symbols, symbol};

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

matryona_machines::symbols! {
    /// The lease core's refusals as Symbols, the lease gem's own names.
    pub LeaseRefusals for LeaseRefusal as "a lease refusal" {
        Occupied => "occupied",
        Held => "held",
        NoLease => "no_lease",
        WrongOwner => "wrong_owner",
        StaleToken => "stale_token",
        Unknown => "unknown",
        Expired => "expired",
        ClockRegressed => "clock_regressed",
        InvalidTtl => "invalid_ttl",
        Exhausted => "exhausted",
        Fenced => "fenced",
        OutOfPhase => "out_of_phase",
    }
}

matryona_machines::symbols! {
    /// The election's clock policies as Symbols: `TypeError` for a
    /// non-Symbol, `RangeError` for another Symbol.
    pub ClockPolicies for ClockPolicy as "clock policy" {
        Reject => "reject",
        Clamp => "clamp",
    }
}

/// A refusal's Symbol: a carried lease refusal answers the lease core's own.
pub fn refusal(ruby: &Ruby, refusal: Refusal) -> Symbol {
    match refusal {
        Refusal::NoCandidates => symbol(ruby, "no_candidates"),
        Refusal::Lease(carried) => LeaseRefusals::symbol(ruby, carried),
    }
}
