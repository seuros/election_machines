# frozen_string_literal: true

require_relative 'election_machines/version'

# Every value mirrors the core type of the same name, field for field, and
# every method takes the core's arguments in the core's order. Every instant
# and span (`now`, `ttl`, `expires_at`, `observed`) is Integer nanoseconds.
# Owners are Integers: the core's `OwnerId` is the caller's number.
#
# FencingToken, Lease and Checkpoint are the lease core's types, which the
# election core re-exports. Wherever a method takes one it reads it by its
# readers, so the `lease_machines` gem's values of the same names fit as they
# are; this gem does not depend on that one.
#
# The native side is the authority on every value it reads: one of this gem's
# classes must be exactly that class (a subclass is a TypeError, whatever it
# holds), and every field is validated again through the core constructor, so
# a value forged past `initialize` (`Marshal.load`, a bound
# `Data#initialize`) raises TypeError or RangeError and never reaches the core.
module ElectionMachines

  # A fencing token: the issuing election's scope in the top 16 bits of `raw`
  # above a sequence number counted from 1. Plain data, so it survives a trip
  # over the wire as `raw` and comes back with `from_raw`. A token built here
  # proves nothing by itself: the election still answers :unknown to one it
  # never issued.
  #
  # Tokens of one scope are ordered; tokens of two scopes are not, so there is
  # no `<` or `<=>`. Compare with `order` and `newer_than?`.
  FencingToken = Data.define(:raw) do
    # The core's `FencingToken::from_raw`. RangeError when the sequence number
    # is zero, which no election issues.
    def self.from_raw(raw) = new(raw)

    def initialize(raw:)
      ElectionMachines.__send__(:native_raw, raw)
      super
    end

    # The scope of the election that issued this token.
    def scope = ElectionMachines.__send__(:native_token_scope, self)

    # -1, 0 or 1 against a token of the same scope; nil across scopes.
    def order(other) = ElectionMachines.__send__(:native_token_order, self, other)

    # Whether this token is of `other`'s scope and strictly newer. False across
    # scopes, so its negation does not mean "older or equal".
    def newer_than?(other) = ElectionMachines.__send__(:native_token_newer, self, other)
  end

  # One leadership: who leads, under which token, until when. Built by hand
  # it is held to the rule a granted one is: an Integer owner in 0..2**64-1,
  # a token read by its `raw` reader and kept as this gem's FencingToken,
  # Integer nanoseconds. TypeError for the wrong type, RangeError for a value
  # outside that.
  Lease = Data.define(:owner, :token, :expires_at) do
    def initialize(owner:, token:, expires_at:)
      super(owner:, token: ElectionMachines.__send__(:native_lease, owner, token, expires_at), expires_at:)
    end
  end

  # Committed election state, the core's `Checkpoint`: persist the four fields
  # before exposing a decision, rebuild with `from_parts`. RangeError when
  # `issued` is beyond the scope's token space or `lease` is not the grant
  # that history ends with (it must carry token number `issued` of `scope`).
  #
  # `lease` is read by its readers and kept as this gem's Lease.
  Checkpoint = Data.define(:scope, :issued, :lease, :observed) do
    # The core's `Checkpoint::from_parts`.
    def self.from_parts(scope, issued, lease, observed) = new(scope, issued, lease, observed)

    def initialize(scope:, issued:, lease:, observed:)
      held = ElectionMachines.__send__(:native_checkpoint, scope, issued, lease, observed)
      super(scope:, issued:, lease: held, observed:)
    end
  end

  # The core's `Leadership::Elected`: the smallest candidate now leads under
  # `lease`, which carries a new token. `lease` is read by its readers and
  # kept as this gem's Lease, validated like one.
  Elected = Data.define(:lease) do
    def initialize(lease:) = super(lease: ElectionMachines.__send__(:native_held, lease))

    def incumbent? = false
  end

  # The core's `Leadership::Incumbent`: a leader's unexpired `lease` is still
  # in force. Honoured, and nothing changed. `lease` is read by its readers
  # and kept as this gem's Lease, validated like one.
  Incumbent = Data.define(:lease) do
    def initialize(lease:) = super(lease: ElectionMachines.__send__(:native_held, lease))

    def incumbent? = true
  end

  # Sealed: no subclass can skip the checks above by overriding `initialize`,
  # and no bare `allocate` builds a value that never ran them. Ruby still
  # binds a subclass constant before `inherited` raises; the native side
  # refuses its instances.
  [FencingToken, Lease, Checkpoint, Elected, Incumbent].each do |value|
    value.singleton_class.undef_method(:allocate)
    value.define_singleton_method(:inherited) { |_| raise TypeError, "#{value} cannot be subclassed" }
  end
end

# Native is required: no algorithm fallback. It resolves the classes above
# once, here.
require 'election_machines_native/election_machines_native'

module ElectionMachines
  class << self
    private :native_raw, :native_token_scope, :native_token_order, :native_token_newer, :native_lease,
            :native_held, :native_checkpoint
  end

  # A lease as the native side answers it, `[owner, raw, expires_at]`.
  def self.lease_of(row) = Lease.new(row[0], FencingToken.new(row[1]), row[2])
  private_class_method :lease_of

  # The core's `Election`: one resource, one token scope. `elect` answers
  # Elected or Incumbent when honoured, `heartbeat` and `resign` a Lease, and
  # each a refusal Symbol otherwise. A refusal changes nothing.
  class Election
    class << self
      # A fresh token namespace in scope zero.
      def new = with_scope(0)

      # A fresh token namespace whose tokens carry `scope`, an Integer in
      # 0..65_535. Give each election its own.
      def with_scope(scope) = native_with_scope(scope)

      # The newest committed checkpoint of `scope`, with the clock policy
      # stated again. RangeError when the checkpoint is of another scope.
      def restored(scope, checkpoint, clock_policy)
        native_restored(scope, checkpoint, clock_policy)
      end

      private :native_with_scope, :native_restored
    end

    # :reject or :clamp. The core's builder consumes the election; here it
    # changes this one and answers it.
    def with_clock_policy(clock_policy)
      native_with_clock_policy(clock_policy)
      self
    end

    # The last leadership granted (it may have expired), nil after a resignation.
    def lease
      held = native_lease
      held && ElectionMachines.__send__(:lease_of, held)
    end

    def checkpoint
      scope, issued, held, observed = native_checkpoint
      Checkpoint.new(scope, issued, held && ElectionMachines.__send__(:lease_of, held), observed)
    end

    # Every decision decides on a copy of the election, builds the answer
    # (its `new` runs), then keeps the copy only if this election is still
    # unfrozen (FrozenError) and unchanged (RuntimeError: a re-entrant call
    # changed it). Either way, a raise means nothing was applied.
    #
    # `candidates` is an Array of owner Integers, of any length.
    def elect(candidates, now, ttl) = native_elect(candidates, now, ttl)

    def heartbeat(owner, token, now, ttl) = native_heartbeat(owner, token, now, ttl)

    def resign(owner, token, now) = native_resign(owner, token, now)
  end
end
