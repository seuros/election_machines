# frozen_string_literal: true

require 'test_helper'

# Building an answer runs Ruby (`new`), so a decision is held on a copy and
# kept only once its answer exists: whatever happens in between, a raise
# means nothing was applied.
class KeepTest < Minitest::Test
  include ElectionHelpers

  def setup
    @election = E::Election.with_scope(SCOPE)
  end

  def test_an_answer_whose_new_raises_applies_nothing
    [E::Elected, E::Lease, E::FencingToken].each do |answer|
      before = snapshot(@election)
      AnswerHook.around(answer, -> { raise IOError, 'interrupted' }) do
        assert_raises(IOError) { @election.elect([1], 0, at(5)) }
      end
      assert_equal before, snapshot(@election)
    end
    assert_equal lease(1, token(SCOPE, 1), at(5)), @election.elect([1], 0, at(5)).lease
  end

  def test_an_answer_whose_new_re_enters_applies_only_the_inner_call
    inner = nil
    AnswerHook.around(E::Elected, -> { inner = @election.elect([2], 0, at(5)) }) do
      assert_raises(RuntimeError) { @election.elect([1], 0, at(9)) }
    end
    assert_equal lease(2, token(SCOPE, 1), at(5)), inner.lease
    assert_equal inner.lease, @election.lease
    assert_equal 1, @election.issued
  end

  def test_an_answer_whose_new_freezes_the_receiver_applies_nothing
    before = snapshot(@election)
    AnswerHook.around(E::Lease, -> { @election.freeze }) do
      assert_raises(FrozenError) { @election.elect([1], 0, at(5)) }
    end
    assert_equal before, snapshot(@election)
  end

  def test_heartbeat_and_resign_keep_their_copy
    held = @election.elect([1], 0, at(5)).lease
    [[:heartbeat, 1, held.token, at(1), at(5)], [:resign, 1, held.token, at(1)]].each do |call, *args|
      before = snapshot(@election)
      AnswerHook.around(E::Lease, -> { raise IOError }) do
        assert_raises(IOError) { @election.public_send(call, *args) }
      end
      assert_equal before, snapshot(@election)
    end
  end

  # The binding holds its classes by address: they are pinned, so compaction
  # moves none of them and every path that checks or builds one still works.
  def test_classes_survive_compaction
    GC.compact if GC.respond_to?(:compact)
    elected = @election.elect([1], 0, at(5))
    assert_instance_of E::Elected, elected
    assert_instance_of E::Incumbent, @election.elect([1], at(1), at(5))
    held = elected.lease
    assert_instance_of E::Lease, @election.heartbeat(1, held.token, at(1), at(5))
    assert_instance_of E::Lease, @election.resign(1, held.token, at(2))
    assert_instance_of E::Checkpoint, E::Checkpoint.new(SCOPE, 1, held, 0)
    assert_instance_of E::Election, E::Election.restored(SCOPE, @election.checkpoint, :reject)
  end
end
