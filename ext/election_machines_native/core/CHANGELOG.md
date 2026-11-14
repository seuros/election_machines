# Changelog

## [0.4.0](https://github.com/seuros/election_machines/compare/election-machines-v0.3.0...election-machines-v0.4.0) (2026-11-14)


### ⚠ BREAKING CHANGES

* validate every value through the core, seal values, exact-class reads, clone-then-keep elect/heartbeat/resign

### Features

* validate every value through the core, seal values, exact-class reads, clone-then-keep elect/heartbeat/resign ([43bbf42](https://github.com/seuros/election_machines/commit/43bbf42813a6cc4ca663764c6ee1186eeff626ec))

## [0.3.0](https://github.com/seuros/election_machines/compare/election-machines-v0.2.2...election-machines-v0.3.0) (2026-11-13)


### ⚠ BREAKING CHANGES

* converted to the fleet conventions. One exhaustive Refusal per crate, Result from every operation that can refuse, a refusal changes nothing, provable idempotent repeats are honoured, Ruby mirrors the core. No compatibility aliases. Requires lease-machines >= 0.4. Refusal is { NoCandidates, Lease(LeaseRefusal) }; elect returns Result<Leadership, Refusal>; Ruby answers values and symbols instead of string tuples.

### Features

* carry the lease refusal whole, honour a live incumbent, scoped restorable elections and Integer-nanosecond Ruby values ([7c23feb](https://github.com/seuros/election_machines/commit/7c23feb24c50da07795646ae7b37f7f04072f318))

## [0.2.2](https://github.com/seuros/election_machines/compare/election-machines-v0.2.1...election-machines-v0.2.2) (2026-11-06)


### Bug Fixes

* crates.io keywords and categories for the core ([8c6d66c](https://github.com/seuros/election_machines/commit/8c6d66cc9ff19907ad68dc4043c9e421a66037ad))

## [0.2.1](https://github.com/seuros/election_machines/compare/election-machines-v0.2.0...election-machines-v0.2.1) (2026-11-06)


### Bug Fixes

* keep test sources out of the packaged gem and crate ([27f00f1](https://github.com/seuros/election_machines/commit/27f00f1331d641a7d324ee7e560324a99f958bfa))

## [0.2.0](https://github.com/seuros/election_machines/compare/election-machines-v0.1.0...election-machines-v0.2.0) (2026-11-06)


### Features

* initial extraction ([a804dd3](https://github.com/seuros/election_machines/commit/a804dd3a3c37b0f96acec918e7242475a63cd0c4))
