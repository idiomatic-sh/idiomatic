//! The facts fakes the execution use case's `activation_project` boundary scenarios inject.
use crate::application::file_dag::program::rule_outcome::types::ProjectBatch;
use crate::outbound::facts::errors::FactsError;
use crate::outbound::facts::interface::FactsFinish;
use crate::outbound::facts::interface::FactsWorker;
use crate::outbound::facts::types::ColumnsMut;
use crate::outbound::facts::types::KeptBucket;
use crate::outbound::facts::types::ResidentStability;
use crate::outbound::facts::types::WordsToKeep;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{ColumnsView, FileId, Relation};
use ryo_values::domain::stored_facts::types::Bucket;

/// Compiled frontiers are empty, so no source row or kept bucket may be requested.
pub struct NoFacts;

impl FactsFinish for NoFacts {
    fn kept(&self, _: Bucket) -> Result<KeptBucket, FactsError> {
        panic!("an activation rule has no file frontier to fetch")
    }
}

impl FactsWorker for NoFacts {
    fn resident_stability(&self) -> ResidentStability {
        ResidentStability::Stable
    }
    fn rows(&self, _: Relation) -> Result<ColumnsView<'_>, FactsError> {
        panic!("an activation rule has no project relation to read")
    }
    fn columns_mut(&mut self) -> Result<ColumnsMut<'_>, FactsError> {
        panic!("an activation rule writes no facts")
    }
    fn keep(&self, _: Bucket, _: WordsToKeep<'_>) -> Result<(), FactsError> {
        panic!("an activation rule writes no facts")
    }
    fn release(&mut self) -> Result<(), FactsError> {
        panic!("an activation rule writes no facts")
    }
    fn finished(self: Box<Self>) -> Result<Box<dyn FactsFinish>, FactsError> {
        panic!("an activation rule writes no facts")
    }
}
