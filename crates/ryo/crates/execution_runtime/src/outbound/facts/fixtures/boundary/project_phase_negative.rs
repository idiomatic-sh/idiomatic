//! The facts fakes the execution use case's `project_phase_negative` boundary scenarios inject.
use crate::application::file_dag::program::observation_stage::types::ObservationStage;
use crate::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use crate::application::file_dag::program::project_phase::types::ProjectPhase;
use crate::application::file_dag::program::request_execution::types::RequestExecution;
use crate::outbound::facts::errors::FactsError;
use crate::outbound::facts::interface::FactsFinish;
use crate::outbound::facts::interface::FactsWorker;
use crate::outbound::facts::types::ColumnsMut;
use crate::outbound::facts::types::KeptBucket;
use crate::outbound::facts::types::ResidentStability;
use crate::outbound::facts::types::WordsToKeep;
use ryo_relations::domain::relations::types::{CellKind, FileId};
use ryo_values::domain::{
    files::types::CorpusInput,
    observations::types::{
        BoundObservationInput, ObservationBinding, ObservationBindingName, ObservationColumn,
        ObservationColumnName,
    },
    plan_parts::types::{Input, ObservationBindingId, StepScope},
    plans::types::{Operation, PlanStep},
    records::types::ObservationReadinessLayout,
    stored_facts::types::{Bucket, Digest},
};

/// No compiled rule in these programs has a file frontier or a project relation.
pub struct NoFacts;

impl FactsFinish for NoFacts {
    fn kept(&self, _: Bucket) -> Result<KeptBucket, FactsError> {
        panic!("a submission control has no file frontier to fetch")
    }
}

impl FactsWorker for NoFacts {
    fn resident_stability(&self) -> ResidentStability {
        ResidentStability::Stable
    }
    fn rows(
        &self,
        _: ryo_relations::domain::relations::types::Relation,
    ) -> Result<ryo_relations::domain::relations::types::ColumnsView<'_>, FactsError> {
        panic!("a submission control has no project relation to read")
    }
    fn columns_mut(&mut self) -> Result<ColumnsMut<'_>, FactsError> {
        panic!("a submission control writes no facts")
    }
    fn keep(&self, _: Bucket, _: WordsToKeep<'_>) -> Result<(), FactsError> {
        panic!("a submission control writes no facts")
    }
    fn release(&mut self) -> Result<(), FactsError> {
        panic!("a submission control writes no facts")
    }
    fn finished(self: Box<Self>) -> Result<Box<dyn FactsFinish>, FactsError> {
        panic!("a submission control writes no facts")
    }
}

/// A compiled rule with no file frontier to fetch.
pub struct NoFileFrontiers;

impl FactsFinish for NoFileFrontiers {
    fn kept(&self, _: Bucket) -> Result<KeptBucket, FactsError> {
        panic!("this compiled rule has no file frontier to fetch")
    }
}
