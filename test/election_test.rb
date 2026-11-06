# frozen_string_literal: true
require_relative 'test_helper'
class ElectionTest < Minitest::Test
  def test_nomination_renewal_resignation_and_fencing
    election = ElectionMachines::Election.new
    assert_equal ['no_candidates', nil, nil, nil], election.elect([], 0, 1)
    assert_equal ['elected', 2, 1, 10.0], election.elect([9, 2, 5], 0, 10)
    assert_equal ['incumbent', 2, 1, 10.0], election.elect([1], 1, 10)
    assert_equal ['WrongOwner', nil, nil, nil], election.heartbeat(1, 1, 2, 10)
    assert_equal ['renewed', 2, 1, 13.0], election.heartbeat(2, 1, 3, 10)
    assert_equal ['released', nil, nil, nil], election.resign(2, 1, 4)
    assert_equal ['elected', 1, 2, 15.0], election.elect([1], 5, 10)
    assert_equal ['StaleToken', nil, nil, nil], election.heartbeat(1, 1, 6, 10)
    assert_equal ['ClockRegressed', nil, nil, nil], election.elect([1], 0, 10)
  end

  def test_checkpoint_scope_staleness_and_gc
    assert_raises(TypeError) { ElectionMachines::Checkpoint.new }
    assert_raises(TypeError) { ElectionMachines::Election.allocate }
    a = ElectionMachines::Election.new
    b = ElectionMachines::Election.new
    a.elect([1], 0, 10)
    saved = a.checkpoint
    assert_raises(TypeError) { saved.dup }
    assert_raises(ArgumentError) { b.restore!(saved) }
    GC.start
    GC.compact
    assert a.restore!(saved)
    assert_equal [1, 1, 10.0], a.lease
    assert_raises(ArgumentError) { a.restore!(saved) }
    stale = a.checkpoint
    a.heartbeat(1, 1, 1, 10)
    assert_raises(ArgumentError) { a.restore!(stale) }
  end

  def test_invalid_arguments_do_not_mutate_and_frozen_mutators_reject
    a = ElectionMachines::Election.new
    saved = a.checkpoint
    assert_raises(TypeError) { a.elect([1.1], 0, 10) }
    assert_raises(TypeError) { a.elect(['1'], 0, 10) }
    assert_raises(RangeError) { a.elect([-1], 0, 10) }
    assert_raises(RangeError) { a.elect([2**80], 0, 10) }
    assert_raises(ArgumentError) { a.elect(Array.new(65, 1), 0, 10) }
    [Float::NAN, Float::INFINITY, -1, 1e100].each do |time|
      assert_raises(ArgumentError) { a.elect([1], time, 10) }
      assert_raises(ArgumentError) { a.elect([1], 0, time) }
    end
    assert_raises(ArgumentError) { a.elect([1], 0, 0) }
    assert a.restore!(saved)
    saved = a.checkpoint
    a.freeze
    assert_raises(FrozenError) { a.elect([1], 0, 10) }
    assert_raises(FrozenError) { a.heartbeat(1, 1, 0, 10) }
    assert_raises(FrozenError) { a.resign(1, 1, 0) }
    assert_raises(FrozenError) { a.restore!(saved) }
    assert_nil a.lease
  end
end
