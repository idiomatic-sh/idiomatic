//! What a row delivery answers: complete, partial or deferred to a later phase.
use ryo_values::domain::plan_parts::types::StepScope;

/// Successful row delivery preserves physical uncertainty rather than fabricating an empty table.
pub enum RowEmission {
    /// No rows were delivered: the compiled root belongs to this later phase.
    Deferred {
        phase: StepScope,
    },
    Complete,
    Partial {
        affected: Vec<Option<ryo_relations::domain::relations::types::FileId>>,
    },
}
