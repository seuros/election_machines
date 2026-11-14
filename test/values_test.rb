# frozen_string_literal: true

require 'test_helper'

# A value is valid or does not exist, whoever built it; what a value holds is
# this gem's own class.
class ValuesTest < Minitest::Test
  include ElectionHelpers

  SiblingToken = Data.define(:raw)
  SiblingLease = Data.define(:owner, :token, :expires_at)

  def test_a_lease_built_by_hand_is_validated_like_a_granted_one
    assert_raises(TypeError) { E::Lease.new('x', token(SCOPE, 1), 0) }
    assert_raises(TypeError) { E::Lease.new(1, 5, 0) }
    assert_raises(TypeError) { E::Lease.new(1, token(SCOPE, 1), 1.5) }
    assert_raises(TypeError) { E::Lease.new(1, SiblingToken.new('1'), 0) }
    assert_raises(RangeError) { E::Lease.new(-1, token(SCOPE, 1), 0) }
    assert_raises(RangeError) { E::Lease.new(2**64, token(SCOPE, 1), 0) }
    assert_raises(RangeError) { E::Lease.new(1, token(SCOPE, 1), -1) }
    assert_raises(RangeError) { E::Lease.new(1, token(SCOPE, 1), (2**64) * SECOND) }
    assert_raises(RangeError) { E::Lease.new(1, SiblingToken.new(SCOPE << 48), 0) }
    assert_raises(RangeError) { E::Lease.new(owner: 1, token: token(SCOPE, 1), expires_at: -1) }
    assert_raises(ArgumentError) { E::Lease.new(1, token(SCOPE, 1)) }
  end

  def test_a_lease_holds_this_gems_token
    lease = E::Lease.new(1, SiblingToken.new(token(SCOPE, 1).raw), at(5))
    assert_instance_of E::FencingToken, lease.token
    elected = E::Election.with_scope(SCOPE).elect([1], 0, at(5))
    assert_equal elected.lease, lease
  end

  def test_leadership_is_validated_and_holds_this_gems_lease
    foreign = SiblingLease.new(1, SiblingToken.new(token(SCOPE, 1).raw), at(5))
    election = E::Election.with_scope(SCOPE)
    elected = election.elect([1], 0, at(5))
    assert_equal elected, E::Elected.new(foreign)
    assert_instance_of E::Lease, E::Elected.new(foreign).lease
    assert_equal election.elect([2], at(1), at(5)), E::Incumbent.new(foreign)
    assert_instance_of E::Lease, E::Incumbent.new(foreign).lease
    [E::Elected, E::Incumbent].each do |kind|
      assert_raises(TypeError) { kind.new(nil) }
      assert_raises(TypeError) { kind.new([1, token(SCOPE, 1), 0]) }
      assert_raises(RangeError) { kind.new(SiblingLease.new(1, SiblingToken.new(0), 0)) }
      assert_raises(RangeError) { kind.new(SiblingLease.new(1, token(SCOPE, 1), -1)) }
    end
  end

  def test_a_value_out_of_the_cores_domain_is_a_range_error_never_an_argument_error
    election = E::Election.with_scope(SCOPE)
    assert_raises(RangeError) { election.with_clock_policy(:ignore) }
    assert_raises(RangeError) { E::Election.restored(SCOPE, election.checkpoint, :later) }
    assert_raises(TypeError) { election.with_clock_policy('clamp') }
    assert_raises(RangeError) { E::FencingToken.new(0) }
    assert_raises(TypeError) { E::FencingToken.new('1') }
    assert_raises(RangeError) { E::Checkpoint.new(SCOPE, 2**48, nil, 0) }
    assert_raises(TypeError) { E::Checkpoint.new(SCOPE, 0, :lease, 0) }
  end

  def test_malformed_candidates_raise_before_the_election_changes
    election = E::Election.with_scope(SCOPE)
    before = election.checkpoint
    assert_raises(TypeError) { election.elect([1, 'x'], 0, at(5)) }
    assert_raises(RangeError) { election.elect([1, -1], 0, at(5)) }
    assert_raises(TypeError) { election.elect(1, 0, at(5)) }
    assert_equal before, election.checkpoint
  end
end
