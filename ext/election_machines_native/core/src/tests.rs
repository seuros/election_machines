use super::*;
fn at(t: u64) -> Duration {
    Duration::from_secs(t)
}

#[test]
fn nomination_is_deterministic_and_incumbency_is_not_membership() {
    let mut election = Election::default();
    let Leadership::Elected(first) =
        election.elect(&[OwnerId::new(9), OwnerId::new(2)], at(0), at(5))
    else {
        panic!("expected election");
    };
    assert_eq!(first.owner(), OwnerId::new(2));
    assert_eq!(
        election.elect(&[OwnerId::new(1)], at(1), at(5)),
        Leadership::Incumbent(first)
    );
    assert_eq!(election.elect(&[], at(1), at(5)), Leadership::NoCandidates);
    let Leadership::Elected(next) = election.elect(&[OwnerId::new(1)], at(5), at(5)) else {
        panic!("expected takeover");
    };
    assert!(next.token() > first.token());
}

#[test]
fn heartbeat_and_recovery_preserve_authority_contract() {
    let (lease, checkpoint) = {
        let mut election = Election::default();
        let Leadership::Elected(lease) = election.elect(&[OwnerId::new(1)], at(1), at(5)) else {
            panic!("expected election");
        };
        assert_eq!(
            election.heartbeat(OwnerId::new(2), lease.token(), at(2), at(5)),
            Decision::Rejected(Rejection::WrongOwner)
        );
        (lease, election.checkpoint())
    };
    let mut restored = Election::restore(checkpoint);
    assert_eq!(
        restored.elect(&[OwnerId::new(2)], at(0), at(5)),
        Leadership::Rejected(Rejection::ClockRegressed)
    );
    assert_eq!(
        restored.resign(lease.owner(), lease.token(), at(3)),
        Decision::Released
    );
    let Leadership::Elected(next) = restored.elect(&[OwnerId::new(2)], at(3), at(5)) else {
        panic!("expected election");
    };
    assert!(next.token() > lease.token());
}
