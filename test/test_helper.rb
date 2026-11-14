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

# Runs a hook once in place of the next `new` on a class, before the real
# one: an answer class whose construction raises, re-enters or freezes.
module AnswerHook
  HOOKS = {} # rubocop:disable Style/MutableConstant

  def new(...)
    AnswerHook::HOOKS.delete(self)&.call
    super
  end

  def self.around(klass, hook)
    klass.singleton_class.prepend(self) unless klass.singleton_class.ancestors.include?(self)
    HOOKS[klass] = hook
    yield
  ensure
    HOOKS.delete(klass)
  end
end

module ElectionHelpers
  # Builds a value past `initialize`: Marshal.load skips it. `from` and `to`
  # are the marshalled bytes of one field, before and after.
  def forge(value, from, to)
    bytes = Marshal.dump(value)
    raise ArgumentError, 'field not found' unless bytes.include?(from)

    Marshal.load(bytes.sub(from, to)) # rubocop:disable Security/MarshalLoad
  end
end
