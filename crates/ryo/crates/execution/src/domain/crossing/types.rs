//! Admitted cross-phase publication transfers; no raw-input fallback.
use ryo_values::domain::stored_facts::types::ArenaBytes;
/// A portable named output. Readiness and words agree: only Complete carries words.
#[derive(Clone, Debug)]
pub struct PackedObservedRoot {
    pub output: ryo_values::domain::observations::types::ObservationIndex,
    pub words: Option<ryo_values::domain::stored_facts::types::KeptWords>,
    pub readiness: ryo_values::domain::observations::types::ProducerReadiness,
    /// Every arena actually referenced by encoded places, including inherited re-exports.
    pub arenas: Vec<ryo_relations::domain::relations::types::WorkerNumber>,
}
/// One closure owns its reserved arena and returns only owned bytes/words.
#[derive(Clone, Debug)]
pub struct CrossPublicationTransfer {
    pub worker: ryo_relations::domain::relations::types::WorkerNumber,
    pub arena: ArenaBytes,
    pub roots: Vec<PackedObservedRoot>,
}

/// One required source that cannot provide rows; never represented by an empty Table.
#[derive(Clone, Debug)]
pub(crate) struct RefusedFrontier {
    pub input: ryo_values::domain::plan_parts::types::Input,
    pub error: crate::domain::operating::errors::OperatingError,
}
