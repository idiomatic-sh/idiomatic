//! What a row delivery answers: complete, partial or deferred to a later phase.
use ryo_values::domain::plan_parts::types::StepScope;

/// Successful row delivery preserves physical uncertainty rather than fabricating an empty table.
pub enum RowEmission {
    /// No rows were delivered: the compiled root belongs to this later phase.
    Deferred {
        /// The phase the root belongs to.
        phase: StepScope,
    },
    /// Every row was delivered.
    Complete,
    /// Rows were delivered from partial inputs.
    Partial {
        /// The files whose rows may be missing; none stands for the repository as a whole.
        affected: Vec<Option<ryo_relations::domain::relations::types::FileId>>,
    },
}
