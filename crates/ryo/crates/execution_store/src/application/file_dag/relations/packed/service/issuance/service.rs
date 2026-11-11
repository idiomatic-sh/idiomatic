//! Owner-dependent admission logic lives below its private data representation.
use super::super::*;
use crate::application::file_dag::derived::ledger::types::{
    BackingAddress, BackingKey, BackingTag, ByteCount, UnitCount,
};
use crate::application::file_dag::relations::service::*;
impl RelationOwner {
    /// Retain the actual encoded words and every referenced existing arena backing.
    /// This adds no optional cache or admission policy; it is an ordinary required publication lease.
    pub(crate) fn packed_publication(
        &self,
        words: KeptWords,
        arenas: &[&ArenaBytes],
    ) -> Result<PackedPublication, OperatingError> {
        let words = Arc::new(words);
        let mut backings = Vec::with_capacity(arenas.len() + 1);
        backings.push(Backing {
            key: BackingKey {
                tag: BackingTag(22),
                address: BackingAddress(Arc::as_ptr(&words).addr()),
            },
            retention_units: UnitCount(0),
            bytes: ByteCount(
                size_of::<KeptWords>() + 2 * size_of::<usize>() + words.0.0.capacity(),
            ),
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
        let lease = self.derived.required_lease(&backings)?;
        Ok(PackedPublication {
            _words: words,
            _lease: lease,
        })
    }
}
