//! Data of a validated restored frontier; behaviour lives beside it in `restored.rs`.
use crate::application::file_dag::program::admission::types::ProducerRoot;
use crate::application::file_dag::program::admission::types::RequiredPartitions;
use ryo_execution_domain::domain::completion::types::CoverageState;
use ryo_execution_domain::domain::interpreted_program::types::interpreted_program::value::InterpretedProgram;
use ryo_relations::domain::relations::types::CellKind;
use std::sync::Arc;

/// A restored frontier the restoration owner validated: only `validate_restored_frontier` makes one,
/// and no code outside the engine can name it.
///
/// ```compile_fail,E0603
/// use ryo_execution_runtime::application::file_dag::program::restored::types::ValidatedFrontier;
/// ```
#[derive(Clone)]
pub(crate) struct ValidatedFrontier {
    pub(super) program: Arc<InterpretedProgram>,
    pub(super) generation: ryo_execution_store::application::file_dag::relations::generation::service::ResidentGeneration,
    pub(super) root: ProducerRoot,
    pub(super) partition: RequiredPartitions,
    pub(super) coverage: CoverageState,
    pub(super) kinds: Box<[CellKind]>,
    pub(super) payload: Arc<crate::outbound::facts::types::KeptBucket>,
}

impl PartialEq for ValidatedFrontier {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.program, &other.program)
            && self.generation.same(&other.generation).is_ok()
            && self.root == other.root
            && self.partition == other.partition
            && self.coverage == other.coverage
            && self.kinds == other.kinds
            && self.payload == other.payload
    }
}
impl Eq for ValidatedFrontier {}
