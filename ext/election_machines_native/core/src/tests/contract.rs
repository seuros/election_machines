use core::cmp::Ordering;

use super::{at, elected, owners, refuses, token, unchanged};
use crate::{Election, Leadership, Lease, LeaseRefusal, OwnerId, Refusal};

#[test]
fn the_smallest_candidate_is_elected_under_a_scoped_token() {
    let mut election = Election::with_scope(7);
    let lease = elected(&mut election, &[9, 2, 5], 0, 10);
    assert_eq!(lease.owner(), OwnerId::new(2));
    assert_eq!(lease.token(), token(7, 1));
    assert_eq!(lease.token().scope(), 7);
    assert_eq!(lease.expires_at(), at(10));
    assert_eq!((election.scope(), election.issued()), (7, 1));
    assert_eq!(election.lease(), Some(lease));
    assert_eq!(Election::new(), Election::with_scope(0));
    assert_eq!(Election::default(), Election::new());
}

#[test]
fn a_live_incumbent_is_honoured_and_nothing_changes() {
    let mut election = Election::new();
    let first = elected(&mut election, &[9, 2], 0, 5);
    // A smaller candidate, the incumbent alone, or a list without it: the
    // lease in force answers every one of them.
    for candidates in [&[1][..], &[2], &[9, 3]] {
        let answer = unchanged(&mut election, |e| {
            e.elect(&owners(candidates), at(1), at(50))
        });
        assert_eq!(answer, Ok(Leadership::Incumbent(first)));
        assert_eq!(answer.map(Leadership::lease), Ok(first));
        assert_eq!(answer.map(Leadership::is_incumbent), Ok(true));
    }
    let next = elected(&mut election, &[1], 5, 5);
    assert_eq!(next.token().order(first.token()), Some(Ordering::Greater));
    assert!(!Leadership::Elected(next).is_incumbent());
}

#[test]
fn elect_refusals_change_nothing() {
    let mut election = Election::new();
    refuses(&mut election, LeaseRefusal::InvalidTtl, |e| {
        e.elect(&owners(&[1]), at(0), at(0))
    });
    refuses(&mut election, LeaseRefusal::InvalidTtl, |e| {
        e.elect(&owners(&[1]), at(1), core::time::Duration::MAX)
    });
    elected(&mut election, &[1], 5, 5);
    refuses(&mut election, LeaseRefusal::ClockRegressed, |e| {
        e.elect(&owners(&[2]), at(4), at(5))
    });
    // An empty list is refused first, whatever the clock or the incumbent.
    refuses(&mut election, Refusal::NoCandidates, |e| {
        e.elect(&[], at(0), at(5))
    });
    refuses(&mut election, Refusal::NoCandidates, |e| {
        e.elect(&[], at(6), at(5))
    });
}

#[test]
fn heartbeat_renews_and_its_refusals_change_nothing() {
    let mut election = Election::with_scope(3);
    refuses(&mut election, LeaseRefusal::Unknown, |e| {
        e.heartbeat(OwnerId::new(1), token(3, 1), at(0), at(5))
    });
    let lease = elected(&mut election, &[1], 1, 5);
    let (owner, held) = (lease.owner(), lease.token());
    refuses(&mut election, LeaseRefusal::WrongOwner, |e| {
        e.heartbeat(OwnerId::new(2), held, at(2), at(5))
    });
    refuses(&mut election, LeaseRefusal::Unknown, |e| {
        e.heartbeat(owner, token(4, 1), at(2), at(5))
    });
    refuses(&mut election, LeaseRefusal::Unknown, |e| {
        e.heartbeat(owner, token(3, 2), at(2), at(5))
    });
    refuses(&mut election, LeaseRefusal::InvalidTtl, |e| {
        e.heartbeat(owner, held, at(2), at(0))
    });
    refuses(&mut election, LeaseRefusal::ClockRegressed, |e| {
        e.heartbeat(owner, held, at(0), at(5))
    });
    let renewed = election.heartbeat(owner, held, at(3), at(5));
    assert_eq!(renewed.map(Lease::expires_at), Ok(at(8)));
    // A repeat with a shorter TTL is honoured and loses no time.
    let repeat = election.heartbeat(owner, held, at(3), at(1));
    assert_eq!(repeat, renewed);
    refuses(&mut election, LeaseRefusal::Expired, |e| {
        e.heartbeat(owner, held, at(8), at(5))
    });
    let next = elected(&mut election, &[1], 8, 5);
    refuses(&mut election, LeaseRefusal::StaleToken, |e| {
        e.heartbeat(owner, held, at(9), at(5))
    });
    assert_eq!(election.lease(), Some(next));
}

#[test]
fn resign_answers_the_lease_given_up_and_a_repeat_is_refused() {
    let mut election = Election::new();
    let lease = elected(&mut election, &[4], 0, 5);
    refuses(&mut election, LeaseRefusal::WrongOwner, |e| {
        e.resign(OwnerId::new(5), lease.token(), at(1))
    });
    refuses(&mut election, LeaseRefusal::Unknown, |e| {
        e.resign(lease.owner(), token(1, 1), at(1))
    });
    assert_eq!(
        election.resign(lease.owner(), lease.token(), at(1)),
        Ok(lease)
    );
    assert_eq!(election.lease(), None);
    assert_eq!(election.issued(), 1);
    refuses(&mut election, LeaseRefusal::NoLease, |e| {
        e.resign(lease.owner(), lease.token(), at(1))
    });
    refuses(&mut election, LeaseRefusal::NoLease, |e| {
        e.heartbeat(lease.owner(), lease.token(), at(1), at(5))
    });
    let next = elected(&mut election, &[4], 1, 5);
    assert!(next.token().is_newer_than(lease.token()));
    refuses(&mut election, LeaseRefusal::Expired, |e| {
        e.resign(next.owner(), next.token(), at(6))
    });
}

#[test]
fn a_lease_refusal_is_carried_whole() {
    use core::error::Error;
    let every = [
        LeaseRefusal::Occupied,
        LeaseRefusal::Held,
        LeaseRefusal::NoLease,
        LeaseRefusal::WrongOwner,
        LeaseRefusal::StaleToken,
        LeaseRefusal::Unknown,
        LeaseRefusal::Expired,
        LeaseRefusal::ClockRegressed,
        LeaseRefusal::InvalidTtl,
        LeaseRefusal::Exhausted,
        LeaseRefusal::Fenced,
        LeaseRefusal::OutOfPhase,
    ];
    for lease in every {
        let carried = Refusal::from(lease);
        assert_eq!(carried, Refusal::Lease(lease));
        assert_eq!(std::format!("{carried}"), "the lease authority refused");
        let source = carried.source().map(std::string::ToString::to_string);
        assert_eq!(source, Some(std::format!("{lease}")));
        assert!(carried.source().and_then(Error::source).is_none());
    }
    let own: &dyn Error = &Refusal::NoCandidates;
    assert_eq!(std::format!("{own}"), "the candidate list is empty");
    assert!(own.source().is_none());
}
