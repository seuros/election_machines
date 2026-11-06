# election-machines

> **AMS LeLann**, NCC-4206, *LeLann*-class command relay.

Engine-room flight manual: model command handovers through one lease authority.

`no_std` lease-backed owner nomination under one authoritative history.
Among participating candidates, the smallest numeric ID is nominated. An
unexpired incumbent remains leader even if absent from the current nomination
list; this list is not a consensus membership configuration.

All lease-machine persistence, fencing, authentication, and clock requirements
apply. A fresh election instance is a fresh namespace, not failover recovery.
Empty candidate lists make no authority transition. Heartbeats renew a lease;
they are not an asynchronous failure detector. This policy is **not Raft,
quorum voting, or partition-safe leader election without an external serialized
durable authority**. Raft elections remain in `consensus-machines`.
