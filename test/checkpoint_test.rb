# frozen_string_literal: true

require_relative 'test_helper'
require 'json'

# Persisting an election across a process restart, and the sibling gem's values.
class CheckpointTest < Minitest::Test
  include ElectionHelpers

  # What the `lease_machines` gem's values look like to this one: other
  # classes with the same readers.
  SiblingToken = Data.define(:raw)
  SiblingLease = Data.define(:owner, :token, :expires_at)
  SiblingCheckpoint = Data.define(:scope, :issued, :lease, :observed)

  # What a store would hold: plain Integers and nil.
  def dump(checkpoint)
    held = checkpoint.lease
    JSON.generate(scope: checkpoint.scope, issued: checkpoint.issued, observed: checkpoint.observed,
                  lease: held && [held.owner, held.token.raw, held.expires_at])
  end

  def load(json)
    row = JSON.parse(json)
    held = row['lease'] && lease(row['lease'][0], E::FencingToken.from_raw(row['lease'][1]), row['lease'][2])
    E::Checkpoint.from_parts(row['scope'], row['issued'], held, row['observed'])
  end

  # The defect this replaces: a refused call staled every saved checkpoint.
  def test_a_refused_call_does_not_stale_a_checkpoint
    election = E::Election.new
    held = election.elect([1], 0, at(10)).lease.token
    saved = election.checkpoint
    assert_refused(:wrong_owner, election) { election.heartbeat(2, held, at(1), at(10)) }
    assert_equal saved, election.checkpoint
    assert_equal saved, E::Election.restored(0, saved, :reject).checkpoint
  end

  # The defect this replaces: a checkpoint restored only the instance it came from.
  def test_a_checkpoint_restores_another_instance_across_a_restart
    election = E::Election.with_scope(SCOPE)
    first = election.elect([1], at(10), at(5)).lease
    saved = dump(election.checkpoint)

    restored = E::Election.restored(SCOPE, load(saved), :reject)
    assert_equal snapshot(election), snapshot(restored)
    assert_unchanged(E::Incumbent.new(first), restored) { restored.elect([2], at(11), at(5)) }
    assert_refused(:clock_regressed, restored) { restored.elect([2], at(9), at(5)) }
    second = restored.elect([2], at(15), at(5)).lease
    assert_equal token(SCOPE, 2), second.token
    assert_refused(:wrong_owner, restored) { restored.heartbeat(1, first.token, at(16), at(5)) }
  end

  def test_restoring_takes_the_scope_and_the_policy_explicitly
    election = E::Election.with_scope(SCOPE).with_clock_policy(:clamp)
    first = election.elect([1], at(10), at(5)).lease
    assert_unchanged(E::Incumbent.new(first), election) { election.elect([2], 0, at(5)) }
    saved = election.checkpoint
    [0, OTHER, 65_535].each do |wrong|
      assert_raises(RangeError) { E::Election.restored(wrong, saved, :clamp) }
    end
    assert_equal :clamp, E::Election.restored(SCOPE, saved, :clamp).clock_policy
    assert_equal :reject, E::Election.restored(SCOPE, saved, :reject).clock_policy
    assert_raises(ArgumentError) { E::Election.restored(SCOPE, saved, :ignore) }
    assert_raises(TypeError) { E::Election.restored(SCOPE, saved, 'clamp') }
    assert_raises(ArgumentError) { election.with_clock_policy(:ignore) }
    assert_raises(ArgumentError) { E::Election.restored(SCOPE, saved) }
  end

  def test_a_checkpoint_is_validated_like_the_core
    held = lease(1, token(SCOPE, 3), at(5))
    assert_equal [SCOPE, 3, held, 0], E::Checkpoint.new(SCOPE, 3, held, 0).to_h.values
    assert_raises(RangeError) { E::Checkpoint.new(SCOPE, 4, held, 0) }
    assert_raises(RangeError) { E::Checkpoint.new(OTHER, 3, held, 0) }
    assert_raises(RangeError) { E::Checkpoint.new(SCOPE, 2**48, nil, 0) }
    assert_raises(TypeError) { E::Checkpoint.new(SCOPE, 3, [1, 3, 5], 0) }
    assert_raises(TypeError) { E::Checkpoint.new(SCOPE, 3, lease(1, 3, at(5)), 0) }
    assert_raises(TypeError) { E::Election.restored(SCOPE, [SCOPE, 0, nil, 0], :reject) }
  end

  def test_the_lease_gems_values_fit_by_their_readers
    held = SiblingLease.new(1, SiblingToken.new((SCOPE << 48) | 1), at(15))
    saved = SiblingCheckpoint.new(SCOPE, 1, held, at(10))
    election = E::Election.restored(SCOPE, saved, :reject)
    assert_equal lease(1, token(SCOPE, 1), at(15)), election.lease
    assert_equal 0, token(SCOPE, 1).order(held.token)
    assert_equal lease(1, token(SCOPE, 1), at(16)), election.heartbeat(1, held.token, at(11), at(5))
    assert_equal lease(1, token(SCOPE, 1), at(16)), election.resign(1, held.token, at(12))
    assert_equal E::Checkpoint.new(SCOPE, 1, nil, at(12)), election.checkpoint
    assert_equal held, E::Checkpoint.new(SCOPE, 1, held, 0).lease
  end
end
