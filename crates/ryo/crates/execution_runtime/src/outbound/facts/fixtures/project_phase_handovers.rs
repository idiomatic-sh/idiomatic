//! The facts fakes the execution runtime's `project_pass` handover cases inject: a store that holds what a case needs, every
//! read logged.
use super::fake::sources_of;
use crate::outbound::facts::errors::FactsError;
use crate::outbound::facts::interface::FactsFinish;
use crate::outbound::facts::types::{
    BucketSource, KeptBucket, KeptSources, KeptWordsLength, KeptWordsStart,
};
use ryo_relations::domain::relations::types::{CellKind, FileId, Value, Words};
use ryo_values::domain::records::types::{
    FrontierAffected, FrontierCoverage, FrontierMissing, FrontierOrdinal, FrontierProof,
    ProducerRule,
};
use ryo_values::domain::rows::types::{RowCount, Table};
use ryo_values::domain::stored_facts::types::{Bucket, Digest, KeptWords};
use ryo_values::domain::table_words::service::{encode_table, pack};
use ryo_values::domain::table_words::types::WordSlice;
use std::sync::{Arc, Mutex, PoisonError};

/// The first bucket of each rule's frontier, by rule: rule 0's file frontier is bucket 0, rule 1's bucket 16.
pub const FIRST_BUCKETS: [Bucket; 2] = [Bucket(0), Bucket(16)];

/// What the phase did, in order: each store read, by bucket, and each rule the guard admitted.
#[derive(Debug, Default)]
pub struct Log(pub Mutex<Vec<String>>);

impl Log {
    /// Adds what happened.
    pub fn push(&self, event: String) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(event);
    }
    /// Everything that happened, in order.
    pub fn events(&self) -> Vec<String> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// What the store holds, as a case needs it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Store {
    /// Nothing in any bucket.
    Empty,
    /// Nothing, and the read of this bucket refused.
    Refusing(Bucket),
    /// Rule 0's file frontier kept by file 0, complete when its provenance is read, partial when its words are.
    CoverageChanged,
    /// Nothing when its provenance is read; when its words are, rule 0's file frontier kept by a file the run does not have.
    OutsiderOnDecode,
}

/// A store that answers `store`, every read logged: `sources` for a provenance read, `kept` for a read of words.
pub struct LoggedKept<'l> {
    /// Where each read is logged.
    pub log: &'l Log,
    /// What it holds.
    pub store: Store,
}

impl LoggedKept<'_> {
    /// What `bucket` holds when its provenance is read, or the refusal of the read.
    fn proven(&self, bucket: Bucket) -> Result<KeptBucket, FactsError> {
        match self.store {
            Store::Refusing(refused) if refused == bucket => Err(FactsError::Store {
                reason: "the kept frontier is unreadable".into(),
            }),
            Store::CoverageChanged if bucket == FIRST_BUCKETS[0] => {
                Ok(one_frame(FileId(0), FrontierCoverage::Complete))
            }
            Store::Empty
            | Store::Refusing(_)
            | Store::CoverageChanged
            | Store::OutsiderOnDecode => Ok(KeptBucket::default()),
        }
    }
}

impl FactsFinish for LoggedKept<'_> {
    fn kept(&self, bucket: Bucket) -> Result<KeptBucket, FactsError> {
        self.log.push(format!("kept {}", bucket.0));
        match self.store {
            Store::CoverageChanged if bucket == FIRST_BUCKETS[0] => Ok(one_frame(
                FileId(0),
                FrontierCoverage::Partial(vec![FrontierAffected::CurrentFile(
                    FrontierMissing::Any,
                )]),
            )),
            Store::OutsiderOnDecode if bucket == FIRST_BUCKETS[0] => {
                Ok(one_frame(FileId(7), FrontierCoverage::Complete))
            }
            Store::Empty
            | Store::Refusing(_)
            | Store::CoverageChanged
            | Store::OutsiderOnDecode => self.proven(bucket),
        }
    }
    fn sources(&self, bucket: Bucket) -> Result<KeptSources, FactsError> {
        self.log.push(format!("sources {}", bucket.0));
        Ok(sources_of(&self.proven(bucket)?))
    }
}

/// A bucket holding one whole frame of rule 0's file frontier, one row naming file 0, kept by `file` with `coverage`.
fn one_frame(file: FileId, coverage: FrontierCoverage) -> KeptBucket {
    let mut words = Words::default();
    encode_table(
        &Table {
            kinds: vec![CellKind::File],
            count: RowCount(1),
            cells: vec![Value::File(FileId(0))],
            ..Table::default()
        },
        &[],
        &mut words,
    );
    let packed = pack(WordSlice(&words.0));
    KeptBucket {
        sources: vec![BucketSource {
            file,
            start: KeptWordsStart(0),
            length: KeptWordsLength(packed.0.len()),
            proof: FrontierProof {
                rule: ProducerRule(0),
                input: FrontierOrdinal(0),
                rule_digest: Digest([0; 16]),
                coverage,
            },
        }],
        words: Arc::new(KeptWords(packed)),
    }
}
