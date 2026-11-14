//! Symbol names for the core's enums. Every match is exhaustive: a new
//! variant fails to compile here until it has a name.
use election_machines::{ClockPolicy, LeaseRefusal, Refusal};

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

matryona_machines::symbols! {
    /// The election's refusals as Symbols: a carried lease refusal answers
    /// under the lease core's own name. Its `NAMES` lists every lease name;
    /// `REFUSALS` keeps the ones an election call can answer.
    pub Refusals for Refusal as "a refusal" => names_only {
        NoCandidates => "no_candidates",
        Lease(inner) => use LeaseRefusals,
    }
}
