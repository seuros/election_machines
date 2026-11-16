# frozen_string_literal: true

require_relative 'test_helper'
require 'open3'
require 'rbconfig'

# The class of the value the extension wraps (Election) is fixed when it
# loads. Nothing a caller does later, redefining the constant or
# `Object.const_get`, reaches an election being built. Each case runs in a
# fresh process: the class is fixed once per process, at load.
class ClassLookupTest < Minitest::Test
  LIB = File.expand_path('../lib', __dir__)

  PRELUDE = <<~RUBY
    require 'election_machines'
    E = ElectionMachines
    second = 1_000_000_000
  RUBY

  # Runs `body` after PRELUDE in a new Ruby; answers its last line of output.
  def isolated(body)
    out, err, status = Open3.capture3(RbConfig.ruby, '-I', LIB, '-e', PRELUDE + body)
    assert status.success?, "exited #{status.exitstatus}: #{err}"
    out.lines.last&.chomp
  end

  # Bug: an Election constant redefined before the first election made it an
  # instance of the new class, without any of the gem's methods.
  def test_a_redefined_election_constant_does_not_change_the_election_class
    assert_equal 'true Elected 2 1', isolated(<<~RUBY)
      original = E::Election
      E.send(:remove_const, :Election)
      E.const_set(:Election, Class.new)
      election = original.new
      elected = election.elect([2], 0, 10 * second)
      puts [election.instance_of?(original), elected.class.name.split('::').last, election.lease.owner,
            election.issued].join(' ')
    RUBY
  end

  # Bug: building the first election ran `Object.const_get`, Ruby code in
  # the middle of a native call. No Ruby runs while one is built now.
  def test_building_an_election_runs_no_const_get
    assert_equal '0 Election Election', isolated(<<~RUBY)
      hits = 0
      Object.define_singleton_method(:const_get) do |*args|
        hits += 1
        super(*args)
      end
      fresh = E::Election.with_scope(7)
      restored = E::Election.restored(7, fresh.checkpoint, :reject)
      puts [hits, fresh.class.name.split('::').last, restored.class.name.split('::').last].join(' ')
    RUBY
  end

  # Bug: a redefined `Object.const_get` that raised hit an `unwrap` in the
  # extension: an uncatchable fatal, and the process exited.
  def test_a_raising_const_get_does_not_kill_the_process
    assert_equal '7 0', isolated(<<~RUBY)
      Object.define_singleton_method(:const_get) { |*| raise IOError, 'lookup ran while building' }
      election = E::Election.with_scope(7)
      puts [election.scope, election.issued].join(' ')
    RUBY
  end
end
