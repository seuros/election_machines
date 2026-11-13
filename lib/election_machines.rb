# frozen_string_literal: true

require_relative 'election_machines/version'
# Native is required: no algorithm fallback.
require 'election_machines_native/election_machines_native'

# Every value mirrors the core type of the same name, field for field, and
# every method takes the core's arguments in the core's order. Every instant
# and span (`now`, `ttl`, `expires_at`, `observed`) is Integer nanoseconds.
# Owners are Integers: the core's `OwnerId` is the caller's number.
#
# FencingToken, Lease and Checkpoint are the lease core's types, which the
# election core re-exports. Wherever a method takes one it reads it by its
# readers, so the `lease_machines` gem's values of the same names fit as they
# are; this gem does not depend on that one.
module ElectionMachines
  class << self
    private :native_token_scope, :native_token_order, :native_token_newer, :native_checkpoint
  end

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
      ElectionMachines.__send__(:native_token_scope, raw)
      super
    end

    # The scope of the election that issued this token.
    def scope = ElectionMachines.__send__(:native_token_scope, raw)

    # -1, 0 or 1 against a token of the same scope; nil across scopes.
    def order(other) = ElectionMachines.__send__(:native_token_order, raw, Rows.raw(other))

    # Whether this token is of `other`'s scope and strictly newer. False across
    # scopes, so its negation does not mean "older or equal".
    def newer_than?(other) = ElectionMachines.__send__(:native_token_newer, raw, Rows.raw(other))
  end

  # One leadership: who leads, under which token, until when.
  Lease = Data.define(:owner, :token, :expires_at)

  # Committed election state, the core's `Checkpoint`: persist the four fields
  # before exposing a decision, rebuild with `from_parts`. RangeError when
  # `issued` is beyond the scope's token space or `lease` is not the grant
  # that history ends with (it must carry token number `issued` of `scope`).
  Checkpoint = Data.define(:scope, :issued, :lease, :observed) do
    # The core's `Checkpoint::from_parts`.
    def self.from_parts(scope, issued, lease, observed) = new(scope, issued, lease, observed)

    def initialize(scope:, issued:, lease:, observed:)
      ElectionMachines.__send__(:native_checkpoint, [scope, issued, lease && Rows.lease(lease), observed])
      super
    end
  end

  # The core's `Leadership::Elected`: the smallest candidate now leads under
  # `lease`, which carries a new token.
  Elected = Data.define(:lease) do
    def incumbent? = false
  end

  # The core's `Leadership::Incumbent`: a leader's unexpired `lease` is still
  # in force. Honoured, and nothing changed.
  Incumbent = Data.define(:lease) do
    def incumbent? = true
  end

  # Values to and from the rows the native extension trades in. Tokens, leases
  # and checkpoints are read by their readers, whichever gem defined them.
  module Rows
    module_function

    def read(value, reader, what)
      raise TypeError, "expected #{what} (no ##{reader})" unless value.respond_to?(reader)

      value.public_send(reader)
    end

    def raw(token) = read(token, :raw, 'a fencing token')

    def lease(lease)
      what = 'a lease'
      [read(lease, :owner, what), raw(read(lease, :token, what)), read(lease, :expires_at, what)]
    end

    def to_lease(row) = Lease.new(row[0], FencingToken.new(row[1]), row[2])

    def checkpoint(checkpoint)
      what = 'a checkpoint'
      held = read(checkpoint, :lease, what)
      [read(checkpoint, :scope, what), read(checkpoint, :issued, what), held && lease(held),
       read(checkpoint, :observed, what)]
    end
  end
  private_constant :Rows

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
        native_restored(scope, Rows.checkpoint(checkpoint), clock_policy)
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
      held && Rows.to_lease(held)
    end

    def checkpoint
      scope, issued, held, observed = native_checkpoint
      Checkpoint.new(scope, issued, held && Rows.to_lease(held), observed)
    end

    # `candidates` is an Array of owner Integers, of any length.
    def elect(candidates, now, ttl)
      answer = native_elect(candidates, now, ttl)
      return answer if answer.is_a?(Symbol)

      incumbent, held = answer
      (incumbent ? Incumbent : Elected).new(Rows.to_lease(held))
    end

    def heartbeat(owner, token, now, ttl) = decided(native_heartbeat(owner, Rows.raw(token), now, ttl))

    def resign(owner, token, now) = decided(native_resign(owner, Rows.raw(token), now))

    private

    def decided(answer) = answer.is_a?(Symbol) ? answer : Rows.to_lease(answer)
  end
end
