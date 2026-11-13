//! Owner-dependent admission logic lives below its private data representation.
use super::super::*;
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
        let backings = crate::application::file_dag::accounting::packed::service::packed_backings(
            &words, arenas,
        );
        let lease = self.derived.required_lease(&backings)?;
        Ok(PackedPublication {
            _words: words,
            _lease: lease,
        })
    }
}
