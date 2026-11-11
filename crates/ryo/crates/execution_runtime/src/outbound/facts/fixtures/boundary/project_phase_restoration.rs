//! The facts fakes the execution use case's `project_phase_restoration` boundary scenarios inject.
use super::project_phase_source_facts::CapturedFacts;
use crate::application::file_dag::program::file_session::types::CaptureContext;
use crate::application::file_dag::program::file_session::types::FileInput;
use crate::application::file_dag::program::file_session::types::SourceLanguage;
use crate::application::file_dag::program::file_session::types::SourceRows;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use crate::outbound::facts::errors::FactsError;
use crate::outbound::facts::interface::FactsFinish;
use crate::outbound::facts::types::KeptBucket;
use crate::outbound::facts::types::ResidentStability;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{FileId, Relation, Value};
use ryo_relations::domain::relations::types::{Words, WorkerNumber};
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::{
    files::types::CorpusInput,
    runs::types::{RuleApplicability, SkipReason, Skipped},
};
use std::sync::atomic::{AtomicUsize, Ordering};

/// The restored producer frontier and, in its own bucket, the consumer's file frontier.
pub struct ConsumerFrontier<'a> {
    pub producer: &'a CapturedFacts,
    pub consumer: &'a CapturedFacts,
}

impl FactsFinish for ConsumerFrontier<'_> {
    fn kept(&self, bucket: Bucket) -> Result<KeptBucket, FactsError> {
        if bucket == Bucket(10) {
            return self.consumer.kept(bucket);
        }
        self.producer.kept(bucket)
    }
}
