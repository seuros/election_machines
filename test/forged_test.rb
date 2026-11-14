# frozen_string_literal: true

require 'test_helper'

# Sealing stops accidents; the native side is the authority. A value forged
# past `initialize` raises TypeError or RangeError wherever it is read, and
# never reaches the core.
class ForgedTest < Minitest::Test
  include ElectionHelpers

  VALUES = [E::FencingToken, E::Lease, E::Checkpoint, E::Elected, E::Incumbent].freeze

  def test_every_value_class_is_sealed
    VALUES.each do |value|
      assert_raises(NoMethodError) { value.allocate }
      assert_raises(TypeError) { Class.new(value) }
    end
  end

  # A named subclass still raises, but Ruby has bound the constant first: it
  # leaks. That is why sealing is not the authority.
  def test_a_named_subclass_raises_but_its_constant_leaks
    leaked_token
    assert ForgedTest.const_defined?(:LeakedToken, false)
    assert_operator ForgedTest::LeakedToken, :<, E::FencingToken
  end

  # A named subclass of FencingToken: `class` raises TypeError, yet the
  # constant is bound. Given a `Data#initialize` bound past the check, it
  # builds tokens holding anything.
  def leaked_token
    unless ForgedTest.const_defined?(:LeakedToken, false)
      source = 'class ForgedTest::LeakedToken < E::FencingToken; end'
      assert_raises(TypeError) { eval(source) } # rubocop:disable Security/Eval
    end
    klass = ForgedTest::LeakedToken
    klass.class_eval do
      define_method(:initialize) { |**fields| Data.instance_method(:initialize).bind_call(self, **fields) }
    end
    klass
  end

  def test_a_subclass_instance_is_refused_whatever_it_holds
    election = E::Election.with_scope(SCOPE)
    lease = election.elect([1], 0, at(5)).lease
    [0, lease.token.raw].each do |raw|
      forged = leaked_token.new(raw:)
      before = snapshot(election)
      assert_raises(TypeError) { election.heartbeat(1, forged, at(1), at(5)) }
      assert_raises(TypeError) { election.resign(1, forged, at(1)) }
      assert_raises(TypeError) { E::Lease.new(1, forged, 0) }
      assert_raises(TypeError) { lease.token.order(forged) }
      assert_raises(TypeError) { forged.scope }
      assert_equal before, snapshot(election)
    end
  end

  def test_a_marshalled_value_is_validated_again
    election = E::Election.new
    election.elect([1], 0, at(5))
    forged = forge(E::FencingToken.new(1), "i\x06".b, "i\x00".b)
    assert_equal 0, forged.raw
    before = snapshot(election)
    assert_raises(RangeError) { election.heartbeat(1, forged, 0, at(5)) }
    assert_raises(RangeError) { election.resign(1, forged, 0) }
    assert_equal before, snapshot(election)
    lease = forge(E::Lease.new(1, E::FencingToken.new(1), 77), "i\x52".b, "i\xFA".b)
    assert_raises(RangeError) { E::Elected.new(lease) }
    assert_raises(RangeError) { E::Checkpoint.new(0, 1, lease, 0) }
    saved = forge(E::Checkpoint.new(0, 0, nil, 77), "i\x52".b, "i\xFA".b)
    assert_raises(RangeError) { E::Election.restored(0, saved, :reject) }
  end

  # A singleton class on the instance itself does not change what it is.
  def test_a_value_with_its_own_singleton_class_is_still_this_gems
    held = token(SCOPE, 1).tap(&:singleton_class)
    assert_equal E::Lease.new(1, token(SCOPE, 1), 0), E::Lease.new(1, held, 0)
    election = E::Election.with_scope(SCOPE)
    lease = election.elect([1], 0, at(5)).lease.tap(&:singleton_class)
    assert_equal lease, E::Elected.new(lease).lease
  end

  # This gem's values are read from the struct, never through their readers:
  # a redefined reader is ignored (a sibling's value is still read by its).
  def test_a_redefined_reader_is_ignored
    election = E::Election.with_scope(SCOPE)
    held = election.elect([1], 0, at(5)).lease
    saved = E::Checkpoint.new(SCOPE, 1, held, 0)
    fencing = held.token
    readers = { E::FencingToken => %i[raw], E::Lease => %i[owner token expires_at],
                E::Checkpoint => %i[scope issued lease observed] }
    originals = readers.flat_map { |klass, names| names.map { [klass, it, klass.instance_method(it)] } }
    begin
      originals.each { |klass, name, _| klass.define_method(name) { raise "#{name} reader ran" } }
      renewed = election.heartbeat(1, fencing, at(1), at(5))
      restored = E::Election.restored(SCOPE, saved, :reject)
    ensure
      originals.each { |klass, name, reader| klass.define_method(name, reader) }
    end
    assert_equal at(6), renewed.expires_at
    assert_equal saved, restored.checkpoint
  end
end
