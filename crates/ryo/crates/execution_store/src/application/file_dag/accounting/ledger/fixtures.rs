//! The backings the ledger's proofs charge and release.
use super::types::{Backing, BackingAddress, BackingKey, BackingTag, ByteCount, UnitCount};

/// A backing of `bytes` at `key`, with no retained units.
pub(super) fn backing(key: BackingAddress, bytes: ByteCount) -> Backing {
    Backing {
        key: BackingKey {
            tag: BackingTag(1),
            address: key,
        },
        bytes,
        retention_units: UnitCount(0),
    }
}

/// A backing of `bytes` at `key` that retains `capacity` units.
pub(super) fn positions(key: BackingAddress, bytes: ByteCount, capacity: UnitCount) -> Backing {
    Backing {
        retention_units: capacity,
        ..backing(key, bytes)
    }
}
