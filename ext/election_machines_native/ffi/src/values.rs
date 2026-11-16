//! Values read from Ruby and built for it.
//!
//! Tokens, leases and checkpoints are read and built by
//! `matryona_machines::ruby::fencing` over [`Family`]: this gem's own class
//! read from the struct, a sibling gem's value of the same shape read by its
//! public readers, a subclass refused, each field converted and the value
//! rebuilt through the core constructor, so a forged value raises before it
//! reaches the core.
use election_machines::{Checkpoint, FencingToken, Lease, OwnerId};

use crate::classes::classes;

matryona_machines::fencing_family! {
    /// The election core's lease family (lease-machines' types), for
    /// `matryona_machines::ruby::fencing`.
    pub Family in classes {
        token: FencingToken,
        lease: Lease, owner: OwnerId,
        checkpoint: Checkpoint,
    }
}
