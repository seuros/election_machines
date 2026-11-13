# frozen_string_literal: true

require 'minitest/autorun'
require 'election_machines'

module ElectionHelpers
  E = ElectionMachines
  SECOND = 1_000_000_000
  SCOPE = 7
  OTHER = 8

  # `seconds` as Integer nanoseconds.
  def at(seconds) = seconds * SECOND

  # Token `sequence` of `scope`, as if it had crossed a wire.
  def token(scope, sequence) = E::FencingToken.from_raw((scope << 48) | sequence)

  def lease(owner, held, expires_at) = E::Lease.new(owner, held, expires_at)

  # Everything observable about an election.
  def snapshot(election)
    [election.checkpoint, election.lease, election.clock_policy, election.scope, election.issued]
  end

  # The call answers `refusal` and leaves the election exactly as it was.
  def assert_refused(refusal, election)
    assert_includes E::REFUSALS, refusal
    before = snapshot(election)
    assert_equal refusal, yield
    assert_equal before, snapshot(election), "#{refusal} changed something"
  end

  # The call is honoured with `answer` and still changes nothing.
  def assert_unchanged(answer, election)
    before = snapshot(election)
    assert_equal answer, yield
    assert_equal before, snapshot(election), "#{answer} changed something"
  end
end
