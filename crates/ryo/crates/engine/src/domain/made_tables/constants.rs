//! The constants of the `made_tables` concept.

use ryo_relations::domain::relations::types::Count;

/// How many rows a batch holds: what a pipeline's stages read and write at a time.
pub(crate) const BATCH_ROWS: Count = Count(4096);
