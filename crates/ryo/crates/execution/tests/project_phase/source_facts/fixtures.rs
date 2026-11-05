//! Injected immutable source rows and the exact file frontier captured by the public owner.
use ryo_execution::inbound::use_cases::execution::types::{CapturedFrontier, facts};
use ryo_relations::domain::relations::{
    service::kinds,
    types::{CellKind, FileId, Offset, Relation, Span, Value},
};
use ryo_relations::domain::relations::{
    service::to_bits,
    types::{ColumnsView, FactColumn},
};
use ryo_values::domain::stored_facts::types::{Bucket, KeptWords, PackedWords};
use std::sync::atomic::{AtomicUsize, Ordering};

pub(in super::super) struct ProjectFacts {
    relation: Relation,
    columns: Vec<FactColumn>,
    pub reads: AtomicUsize,
}
impl ProjectFacts {
    /// A real declared schema with no observed project rows.
    pub fn empty(relation: Relation) -> Self {
        let mut facts = Self::new(relation);
        for column in &mut facts.columns {
            column.0.clear();
        }
        facts
    }
    pub fn new(relation: Relation) -> Self {
        let columns = kinds(relation)
            .iter()
            .map(|kind| {
                let value = match kind {
                    CellKind::File => Value::File(FileId(0)),
                    CellKind::Symbol => {
                        Value::Symbol(ryo_relations::domain::terms::types::Symbol(0))
                    }
                    CellKind::Span => Value::Span(Span {
                        start: Offset(0),
                        end: Offset(0),
                    }),
                    _ => panic!("project fixture follows its actual catalog schema"),
                };
                FactColumn(vec![to_bits(value).0])
            })
            .collect();
        Self {
            relation,
            columns,
            reads: AtomicUsize::new(0),
        }
    }
}
fn unsupported() -> facts::FactsError {
    facts::FactsError::Store {
        reason: "immutable project fixture does not write facts".into(),
    }
}
impl facts::FactsWorker for ProjectFacts {
    fn resident_stability(&self) -> facts::ResidentStability {
        facts::ResidentStability::Stable
    }
    fn rows(&self, relation: Relation) -> Result<ColumnsView<'_>, facts::FactsError> {
        assert_eq!(relation, self.relation);
        self.reads.fetch_add(1, Ordering::SeqCst);
        Ok(ColumnsView {
            kinds: kinds(relation),
            columns: &self.columns,
        })
    }
    fn columns_mut(&mut self) -> Result<facts::ColumnsMut<'_>, facts::FactsError> {
        Err(unsupported())
    }
    fn keep(&self, _: Bucket, _: facts::WordsToKeep<'_>) -> Result<(), facts::FactsError> {
        Err(unsupported())
    }
    fn release(&mut self) -> Result<(), facts::FactsError> {
        Err(unsupported())
    }
    fn finished(self: Box<Self>) -> Result<Box<dyn facts::FactsFinish>, facts::FactsError> {
        Err(unsupported())
    }
}

pub(in super::super) struct CapturedFacts {
    pub captures: Vec<(FileId, CapturedFrontier)>,
    pub reads: AtomicUsize,
}
impl facts::FactsFinish for CapturedFacts {
    fn kept(&self, bucket: Bucket) -> Result<facts::KeptBucket, facts::FactsError> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        let mut words = vec![];
        let mut sources = vec![];
        for (file, capture) in &self.captures {
            assert_eq!(bucket, capture.bucket);
            sources.push(facts::BucketSource {
                file: *file,
                start: words.len(),
                length: capture.words.0.len(),
                proof: capture.proof.clone(),
            });
            words.extend_from_slice(&capture.words.0);
        }
        Ok(facts::KeptBucket {
            words: KeptWords(PackedWords(words)),
            sources,
        })
    }
}
