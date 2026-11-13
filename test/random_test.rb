# frozen_string_literal: true

require_relative 'test_helper'

# Random calls: every refusal changes nothing, and REFUSALS is exactly the set
# of symbols an election answers. :occupied and :held (a live leader is the
# honoured Incumbent) and :fenced and :out_of_phase (the lease holder's) are
# the lease core's symbols that must never come back from here.
class RandomTest < Minitest::Test
  include ElectionHelpers

  LAST = (2**48) - 1

  def setup
    @random = Random.new(0x1ea5e)
    @seen = []
    @incumbents = 0
  end

  # Mostly forward, sometimes back.
  def now
    return @random.rand(@clock + 1) if @random.rand(8).zero?

    @clock += @random.rand(6)
  end

  def ttl = [0, ((2**64) * SECOND) - 1, 1 + @random.rand(9)].fetch([@random.rand(12), 2].min)

  # The token in force, an older one, an unissued one, or another scope's.
  def presented(election)
    issued = election.issued
    current = election.lease ? election.lease.token.raw & LAST : [issued, 1].max
    case @random.rand(8)
    when 0 then token(election.scope, 1 + @random.rand([issued, 1].max))
    when 1 then token(election.scope, [issued + 1, LAST].min)
    when 2 then token(election.scope ^ 1, current)
    else token(election.scope, current)
    end
  end

  def call(election)
    owner = @random.rand(4)
    case @random.rand(10)
    when 0..3 then election.elect(Array.new(@random.rand(4)) { @random.rand(4) }, now, ttl)
    when 4..7 then election.heartbeat(owner, presented(election), now, ttl)
    else election.resign(owner, presented(election), now)
    end
  end

  def step(election)
    before = snapshot(election)
    answer = call(election)
    case answer
    when Symbol then @seen << answer
    when E::Incumbent then @incumbents += 1
    else return
    end
    assert_equal before, snapshot(election), "#{answer.inspect} changed something"
  end

  def run_election(seed)
    scope = (seed % 3) * 0x7fff
    # Every fourth run starts a few tokens short of the end of the space.
    issued = (seed % 4).zero? ? LAST - 3 : seed % 3
    saved = E::Checkpoint.from_parts(scope, issued, nil, 0)
    election = E::Election.restored(scope, saved, seed.even? ? :reject : :clamp)
    @clock = 0
    1_500.times { step(election) }
  end

  def test_refusals_lists_exactly_what_an_election_answers
    (1..16).each { |seed| run_election(seed) }
    assert_operator @incumbents, :>, 0
    assert_empty @seen.uniq - E::REFUSALS, 'answered a symbol REFUSALS does not list'
    assert_empty E::REFUSALS - @seen.uniq, 'REFUSALS lists a symbol no call answered'
  end
end
