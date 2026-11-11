//! The facts fakes the execution use case's `project_phase_rewrites` boundary scenarios inject.
use crate::outbound::facts::errors::FactsError;
use crate::outbound::facts::interface::FactsFinish;
use crate::outbound::facts::types::{BucketSource, KeptBucket};
use ryo_execution_domain::domain::frontier_capture::types::CapturedFrontier;
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stored_facts::types::{KeptWords, PackedWords};
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
