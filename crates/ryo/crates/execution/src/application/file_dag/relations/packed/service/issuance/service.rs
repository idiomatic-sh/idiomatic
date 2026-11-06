//! Owner-dependent admission logic lives below its private data representation.
use super::super::*;
use crate::application::file_dag::relations::publication::types::*;
use crate::application::file_dag::relations::*;
use crate::domain::table_model::collections::representations::publication::PublishedRelation;
impl RelationOwner {
    /// Retain the actual encoded words and every referenced existing arena backing.
    /// This adds no optional cache or admission policy; it is an ordinary required publication lease.
    pub(in crate::application::file_dag) fn packed_publication(
        &self,
        words: KeptWords,
        arenas: &[&ArenaBytes],
    ) -> Result<PackedPublication, OperatingError> {
        let words = Arc::new(words);
        let mut backings = Vec::with_capacity(arenas.len() + 1);
        backings.push(Backing {
            key: (22, Arc::as_ptr(&words) as usize),
            retention_units: 0,
            bytes: size_of::<KeptWords>() + 2 * size_of::<usize>() + words.0.0.capacity(),
        });
        for arena in arenas {
            if arena.0.capacity() != 0 {
                backings.push(Backing {
                    key: (23, arena.0.as_ptr() as usize),
                    bytes: arena.0.capacity(),
                    retention_units: 0,
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
