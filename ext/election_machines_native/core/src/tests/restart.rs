use super::{LAST, at, elected, owners, refuses, token, unchanged};
use crate::{Checkpoint, ClockPolicy, Election, Leadership, LeaseRefusal};

#[test]
fn a_checkpoint_restores_under_its_own_scope_only() {
    let mut election = Election::with_scope(9);
    let lease = elected(&mut election, &[1], 1, 5);
    let saved = election.checkpoint();
    assert_eq!((saved.scope(), saved.issued()), (9, 1));
    assert_eq!(
        Election::restored(8, saved.clone(), ClockPolicy::Reject),
        None
    );
    assert_eq!(
        Election::restored(0, saved.clone(), ClockPolicy::Reject),
        None
    );
    let restored = Election::restored(9, saved, ClockPolicy::Reject);
    assert_eq!(restored, Some(election.clone()));

    let mut restored = restored.unwrap();
    refuses(&mut restored, LeaseRefusal::ClockRegressed, |e| {
        e.elect(&owners(&[2]), at(0), at(5))
    });
    assert_eq!(
        restored.resign(lease.owner(), lease.token(), at(3)),
        Ok(lease)
    );
    let next = elected(&mut restored, &[2], 3, 5);
    assert_eq!(next.token(), token(9, 2));
}

#[test]
fn a_refused_call_leaves_an_earlier_checkpoint_current() {
    let mut election = Election::new();
    let lease = elected(&mut election, &[1], 0, 10);
    let saved = election.checkpoint();
    refuses(&mut election, LeaseRefusal::WrongOwner, |e| {
        e.heartbeat(crate::OwnerId::new(2), lease.token(), at(1), at(10))
    });
    assert_eq!(election.checkpoint(), saved);
}

#[test]
fn the_clock_policy_is_stated_again_and_clamp_never_revives_a_lease() {
    let mut election = Election::with_scope(2).with_clock_policy(ClockPolicy::Clamp);
    assert_eq!(election.clock_policy(), ClockPolicy::Clamp);
    let first = elected(&mut election, &[1], 10, 5);
    // Evaluated at the watermark (10), not at 0: the lease is still live.
    let answer = unchanged(&mut election, |e| e.elect(&owners(&[2]), at(0), at(5)));
    assert_eq!(answer, Ok(Leadership::Incumbent(first)));

    let rejecting = Election::restored(2, election.checkpoint(), ClockPolicy::Reject);
    let mut rejecting = rejecting.unwrap();
    assert_eq!(rejecting.clock_policy(), ClockPolicy::Reject);
    refuses(&mut rejecting, LeaseRefusal::ClockRegressed, |e| {
        e.elect(&owners(&[2]), at(0), at(5))
    });
}

#[test]
fn tokens_never_wrap() {
    let saved = Checkpoint::from_parts(1, LAST - 1, None, at(0)).unwrap();
    let mut election = Election::restored(1, saved, ClockPolicy::Reject).unwrap();
    let last = elected(&mut election, &[1], 0, 5);
    assert_eq!(last.token(), token(1, LAST));
    refuses(&mut election, LeaseRefusal::Exhausted, |e| {
        e.elect(&owners(&[1]), at(5), at(5))
    });
    assert_eq!(Checkpoint::from_parts(1, LAST + 1, None, at(0)), None);
}
