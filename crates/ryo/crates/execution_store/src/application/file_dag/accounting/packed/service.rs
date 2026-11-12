//! Describe the allocations of owned cross-root words once, when they are published, never by hashing rows.
use crate::application::file_dag::accounting::ledger::types::{
    Backing, BackingAddress, BackingKey, BackingTag, ByteCount, UnitCount,
};
use ryo_values::domain::stored_facts::types::{ArenaBytes, KeptWords};
use std::sync::Arc;

/// The words' own allocation, then each existing arena the words reference.
pub(crate) fn packed_backings(words: &Arc<KeptWords>, arenas: &[&ArenaBytes]) -> Vec<Backing> {
    let mut backings = Vec::with_capacity(arenas.len() + 1);
    backings.push(Backing {
        key: BackingKey {
            tag: BackingTag(22),
            address: BackingAddress(Arc::as_ptr(words).addr()),
        },
        retention_units: UnitCount(0),
        bytes: ByteCount(size_of::<KeptWords>() + 2 * size_of::<usize>() + words.0.0.capacity()),
    });
    for arena in arenas {
        if arena.0.capacity() != 0 {
            backings.push(Backing {
                key: BackingKey {
                    tag: BackingTag(23),
                    address: BackingAddress(arena.0.as_ptr().addr()),
                },
                bytes: ByteCount(arena.0.capacity()),
                retention_units: UnitCount(0),
            });
        }
    }
    backings
}
