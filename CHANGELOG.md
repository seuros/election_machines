# Changelog

## [0.5.0](https://github.com/seuros/election_machines/compare/election_machines/v0.4.1...election_machines/v0.5.0) (2026-11-14)


### ⚠ BREAKING CHANGES

* read sibling tokens, leases and checkpoints with own_or_sibling and delegate lease refusal names

### Features

* read sibling tokens, leases and checkpoints with own_or_sibling and delegate lease refusal names ([1b7c5a9](https://github.com/seuros/election_machines/commit/1b7c5a99ae0720478b984cc2693afd6b28c5a26a))

## [0.4.1](https://github.com/seuros/election_machines/compare/election_machines/v0.4.0...election_machines/v0.4.1) (2026-11-14)


### Bug Fixes

* **deps:** require matryona-machines &gt;= 0.3 and build exact-field messages lazily ([2af60e9](https://github.com/seuros/election_machines/commit/2af60e9b11cd79bf3a387edce8e26a442d94b10e))

## [0.4.0](https://github.com/seuros/election_machines/compare/election_machines/v0.3.0...election_machines/v0.4.0) (2026-11-14)


### ⚠ BREAKING CHANGES

* adopt matryona-machines 0.2: shared instant, exact field reads, compiler-checked symbol tables

### Features

* adopt matryona-machines 0.2: shared instant, exact field reads, compiler-checked symbol tables ([b7cbfd2](https://github.com/seuros/election_machines/commit/b7cbfd2adb6edd2a7c5dce7e57c501d7f29a8c2b))

## [0.3.0](https://github.com/seuros/election_machines/compare/election_machines/v0.2.0...election_machines/v0.3.0) (2026-11-14)


### ⚠ BREAKING CHANGES

* validate every value through the core, seal values, exact-class reads, clone-then-keep elect/heartbeat/resign

### Features

* validate every value through the core, seal values, exact-class reads, clone-then-keep elect/heartbeat/resign ([43bbf42](https://github.com/seuros/election_machines/commit/43bbf42813a6cc4ca663764c6ee1186eeff626ec))

## [0.2.0](https://github.com/seuros/election_machines/compare/election_machines/v0.1.5...election_machines/v0.2.0) (2026-11-13)


### ⚠ BREAKING CHANGES

* converted to the fleet conventions. One exhaustive Refusal per crate, Result from every operation that can refuse, a refusal changes nothing, provable idempotent repeats are honoured, Ruby mirrors the core. No compatibility aliases. Requires lease-machines >= 0.4. Refusal is { NoCandidates, Lease(LeaseRefusal) }; elect returns Result<Leadership, Refusal>; Ruby answers values and symbols instead of string tuples.

### Features

* carry the lease refusal whole, honour a live incumbent, scoped restorable elections and Integer-nanosecond Ruby values ([7c23feb](https://github.com/seuros/election_machines/commit/7c23feb24c50da07795646ae7b37f7f04072f318))

## [0.1.5](https://github.com/seuros/election_machines/compare/election_machines/v0.1.4...election_machines/v0.1.5) (2026-11-06)


### Bug Fixes

* extract Wrapper::advance for election revision handling ([09bdd65](https://github.com/seuros/election_machines/commit/09bdd65226a7780bfe980bf19e5145ee2c2f1da0))

## [0.1.4](https://github.com/seuros/election_machines/compare/election_machines/v0.1.3...election_machines/v0.1.4) (2026-11-06)


### Bug Fixes

* require lease-machines 0.3 ([d99d292](https://github.com/seuros/election_machines/commit/d99d2928ba0dd42e3013eb2d13e9a527387703ee))

## [0.1.3](https://github.com/seuros/election_machines/compare/election_machines/v0.1.2...election_machines/v0.1.3) (2026-11-06)


### Bug Fixes

* crates.io keywords and categories for the core ([8c6d66c](https://github.com/seuros/election_machines/commit/8c6d66cc9ff19907ad68dc4043c9e421a66037ad))

## [0.1.2](https://github.com/seuros/election_machines/compare/election_machines/v0.1.1...election_machines/v0.1.2) (2026-11-06)


### Bug Fixes

* keep test sources out of the packaged gem and crate ([27f00f1](https://github.com/seuros/election_machines/commit/27f00f1331d641a7d324ee7e560324a99f958bfa))

## [0.1.1](https://github.com/seuros/election_machines/compare/election_machines-v0.1.0...election_machines/v0.1.1) (2026-11-06)


### Features

* initial extraction ([a804dd3](https://github.com/seuros/election_machines/commit/a804dd3a3c37b0f96acec918e7242475a63cd0c4))
