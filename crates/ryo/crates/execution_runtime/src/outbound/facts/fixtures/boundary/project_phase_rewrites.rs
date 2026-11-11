//! The facts fakes the execution use case's `project_phase_rewrites` boundary scenarios inject.
use crate::application::file_dag::program::file_session::types::CaptureContext;
use crate::application::file_dag::program::file_session::types::FileInput;
use crate::application::file_dag::program::file_session::types::SourceLanguage;
use crate::application::file_dag::program::file_session::types::SourceRows;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use crate::outbound::facts::errors::FactsError;
use crate::outbound::facts::interface::FactsFinish;
use crate::outbound::facts::types::ResidentStability;
use crate::outbound::facts::types::{BucketSource, KeptBucket};
use ryo_execution_domain::domain::frontier_capture::types::CapturedFrontier;
use ryo_language::domain::symbols::types::Symbols;
use ryo_language::domain::symbols::{fixtures::intern, types::ByteSlice};
use ryo_relations::domain::relations::fixtures::node_id;
use ryo_relations::domain::relations::types::{Offset, Position, Span};
use ryo_relations::domain::relations::types::{Words, WorkerNumber};
use ryo_relations::domain::relations::{
    fixtures::kinds,
    types::{CellKind, FileId, Relation, Value},
};
use ryo_values::domain::files::types::{
    CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds,
};
use ryo_values::domain::plans::types::Emit;
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stored_facts::types::{ArenaBytes, KeptWords, PackedWords};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input, StepId, StepScope},
    plans::types::{Operation, PlanStep},
    rows::types::RowCount,
};
use std::sync::Arc;

/// The frontiers the files captured, by bucket.
pub struct CapturedBuckets {
    pub captures: Vec<(FileId, CapturedFrontier)>,
}

impl FactsFinish for CapturedBuckets {
    fn kept(&self, bucket: Bucket) -> Result<KeptBucket, FactsError> {
        let mut words = vec![];
        let mut sources = vec![];
        for (file, capture) in self
            .captures
            .iter()
            .filter(|(_, capture)| capture.bucket == bucket)
        {
            sources.push(BucketSource {
                file: *file,
                start: crate::outbound::facts::types::KeptWordsStart(words.len()),
                length: crate::outbound::facts::types::KeptWordsLength(capture.words.0.len()),
                proof: capture.proof.clone(),
            });
            words.extend_from_slice(&capture.words.0);
        }
        Ok(KeptBucket {
            words: Arc::new(KeptWords(PackedWords(words))),
            sources,
        })
    }
}
