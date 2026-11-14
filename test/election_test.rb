# frozen_string_literal: true

require_relative 'test_helper'

class ElectionTest < Minitest::Test
  include ElectionHelpers

  def test_nomination_renewal_resignation_and_fencing
    election = E::Election.new
    first = lease(2, token(0, 1), at(10))
    assert_equal E::Elected.new(first), election.elect([9, 2, 5], 0, at(10))
    assert_equal first, election.lease
    assert_unchanged(E::Incumbent.new(first), election) { election.elect([1], at(1), at(10)) }
    assert_refused(:wrong_owner, election) { election.heartbeat(1, first.token, at(2), at(10)) }
    assert_equal lease(2, token(0, 1), at(13)), election.heartbeat(2, first.token, at(3), at(10))
    assert_equal lease(2, token(0, 1), at(13)), election.resign(2, first.token, at(4))
    assert_nil election.lease
    assert_refused(:no_lease, election) { election.resign(2, first.token, at(4)) }
    second = election.elect([1], at(5), at(10))
    assert_equal E::Elected.new(lease(1, token(0, 2), at(15))), second
    assert_refused(:stale_token, election) { election.heartbeat(1, first.token, at(6), at(10)) }
    assert_refused(:clock_regressed, election) { election.elect([1], 0, at(10)) }
    assert second.lease.token.newer_than?(first.token)
    refute second.incumbent?
    assert E::Incumbent.new(first).incumbent?
  end

  def test_every_elect_refusal_is_a_symbol_and_changes_nothing
    election = E::Election.with_scope(SCOPE)
    assert_refused(:no_candidates, election) { election.elect([], 0, at(1)) }
    assert_refused(:invalid_ttl, election) { election.elect([1], 0, 0) }
    assert_refused(:invalid_ttl, election) { election.elect([1], 1, ((2**64) * SECOND) - 1) }
    election.elect([1], at(5), at(5))
    assert_refused(:clock_regressed, election) { election.elect([2], at(4), at(5)) }
    assert_refused(:no_candidates, election) { election.elect([], 0, at(1)) }
    almost = E::Checkpoint.from_parts(SCOPE, (2**48) - 1, nil, 0)
    full = E::Election.restored(SCOPE, almost, :reject)
    assert_refused(:exhausted, full) { full.elect([1], 0, at(1)) }
  end

  def test_every_heartbeat_and_resign_refusal_changes_nothing
    election = E::Election.with_scope(SCOPE)
    assert_refused(:unknown, election) { election.heartbeat(1, token(SCOPE, 1), 0, at(5)) }
    held = election.elect([1], at(1), at(5)).lease.token
    [token(OTHER, 1), token(SCOPE, 2), token(0, 1)].each do |foreign|
      assert_refused(:unknown, election) { election.heartbeat(1, foreign, at(2), at(5)) }
      assert_refused(:unknown, election) { election.resign(1, foreign, at(2)) }
    end
    assert_refused(:wrong_owner, election) { election.resign(2, held, at(2)) }
    assert_refused(:invalid_ttl, election) { election.heartbeat(1, held, at(2), 0) }
    assert_refused(:clock_regressed, election) { election.heartbeat(1, held, 0, at(5)) }
    assert_refused(:clock_regressed, election) { election.resign(1, held, 0) }
    assert_refused(:expired, election) { election.heartbeat(1, held, at(6), at(5)) }
    assert_refused(:expired, election) { election.resign(1, held, at(6)) }
    renewed = election.heartbeat(1, held, at(3), at(5))
    assert_equal renewed, election.heartbeat(1, held, at(3), at(1)), 'a repeat never shortens the lease'
    election.elect([3], at(8), at(5))
    assert_refused(:wrong_owner, election) { election.heartbeat(1, held, at(9), at(5)) }
    assert_refused(:stale_token, election) { election.resign(3, held, at(9)) }
  end

  def test_any_number_of_candidates
    election = E::Election.new
    crowd = (100..400).to_a.shuffle(random: Random.new(1))
    assert_equal 100, election.elect(crowd, 0, at(1)).lease.owner
  end

  def test_the_refusal_list_has_no_catch_all
    assert_equal %i[no_candidates no_lease wrong_owner stale_token unknown expired
                    clock_regressed invalid_ttl exhausted], E::REFUSALS
    assert_equal %i[reject clamp], E::CLOCK_POLICIES
    assert_predicate E::REFUSALS, :frozen?
  end

  def test_tokens_are_scoped_values_without_an_order_across_scopes
    one = token(SCOPE, 1)
    assert_equal [SCOPE, (SCOPE << 48) | 1], [one.scope, one.raw]
    assert_equal [-1, 0, 1], [one.order(token(SCOPE, 2)), one.order(one), token(SCOPE, 2).order(one)]
    assert_nil one.order(token(OTHER, 1))
    refute token(OTHER, 9).newer_than?(one)
    refute_respond_to one, :<
    assert_raises(RangeError) { E::FencingToken.from_raw(0) }
    assert_raises(RangeError) { E::FencingToken.from_raw(SCOPE << 48) }
    assert_raises(RangeError) { E::FencingToken.new(2**64) }
    assert_raises(TypeError) { E::FencingToken.new(1.0) }
    assert_raises(TypeError) { one.order(1) }
  end

  def test_malformed_input_raises_and_changes_nothing
    election = E::Election.new
    held = election.elect([1], 0, at(10)).lease.token
    before = snapshot(election)
    [[1.1], ['1'], [nil]].each { |bad| assert_raises(TypeError) { election.elect(bad, 0, at(1)) } }
    assert_raises(TypeError) { election.elect(1, 0, at(1)) }
    [[-1], [2**64]].each { |bad| assert_raises(RangeError) { election.elect(bad, 0, at(1)) } }
    [1.0, '1', nil, Float::NAN].each do |time|
      assert_raises(TypeError) { election.elect([1], time, at(1)) }
      assert_raises(TypeError) { election.heartbeat(1, held, at(1), time) }
      assert_raises(TypeError) { election.resign(1, held, time) }
    end
    [-1, (2**64) * SECOND].each do |time|
      assert_raises(RangeError) { election.elect([1], time, at(1)) }
      assert_raises(RangeError) { election.elect([1], 0, time) }
    end
    assert_raises(TypeError) { election.heartbeat(1, 1, at(1), at(1)) }
    assert_raises(TypeError) { election.resign(1, held.raw, at(1)) }
    assert_raises(TypeError) { election.heartbeat('1', held, at(1), at(1)) }
    assert_raises(TypeError) { E::Election.with_scope('7') }
    assert_raises(RangeError) { E::Election.with_scope(65_536) }
    assert_raises(TypeError) { E::Election.allocate }
    assert_equal before, snapshot(election)
  end

  # The classes of an honoured answer are resolved once at load: removing a
  # constant afterwards changes nothing the binding builds.
  def test_answer_classes_are_resolved_at_load
    %i[Elected Lease FencingToken].each do |name|
      saved = E.send(:remove_const, name)
      begin
        elected = E::Election.new.elect([1], 0, at(10))
        assert_instance_of saved, { Elected: elected, Lease: elected.lease, FencingToken: elected.lease.token }[name]
      ensure
        E.const_set(name, saved)
      end
    end
  end

  def test_a_frozen_election_refuses_every_mutation_and_still_reads
    election = E::Election.new
    held = election.elect([1], 0, at(10)).lease
    election.freeze
    assert_raises(FrozenError) { election.elect([1], 0, at(10)) }
    assert_raises(FrozenError) { election.heartbeat(1, held.token, 0, at(10)) }
    assert_raises(FrozenError) { election.resign(1, held.token, 0) }
    assert_raises(FrozenError) { election.with_clock_policy(:clamp) }
    assert_equal held, election.lease
    assert_equal [0, 1, :reject], [election.scope, election.issued, election.clock_policy]
  end

  # Frozen is checked before any argument is read: a frozen election handed a
  # malformed argument raises FrozenError, as Ruby's own core methods do.
  def test_frozen_is_checked_before_the_arguments
    election = E::Election.new.freeze
    assert_raises(FrozenError) { election.elect(nil, 0, at(10)) }
    assert_raises(FrozenError) { election.elect([1.5], 0, at(10)) }
    assert_raises(FrozenError) { election.heartbeat(1, :token, 0, at(10)) }
    assert_raises(FrozenError) { election.resign(-1, nil, 0) }
    assert_raises(FrozenError) { election.with_clock_policy(:sometimes) }
  end

  # Candidates are read natively from exactly an Array: a subclass, whose
  # iteration it could redefine, is a TypeError.
  def test_candidates_must_be_exactly_an_array
    election = E::Election.new
    assert_raises(TypeError) { election.elect(Class.new(Array).new([1]), 0, at(10)) }
    assert_equal 0, election.issued
    assert_instance_of E::Elected, election.elect([1], 0, at(10))
  end
end
