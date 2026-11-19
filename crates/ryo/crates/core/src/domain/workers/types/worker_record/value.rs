//! One worker's finished file record, lent to ordered delivery only after every staged bucket has its original descriptor.
use ryo_values::domain::records::types::{
    FileOutcome, FrontierCoverage, FrontierProof, Handed, HandedWords, Handing, PayloadLength,
    PayloadOffset, Record, RecordWords, StagedWords, StagingSession,
};
use ryo_values::domain::stored_facts::types::Bucket;
use std::sync::Arc;
/// An ordered bucket and the exact validated proof it shares with other files of this worker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReceiptLayout(Bucket, FrontierProof);
/// The number of staged buckets with consecutive equal-length spans.
#[derive(Clone, Copy, Debug)]
struct ReceiptCount(usize);
#[derive(Clone, Copy, Debug)]
struct StagedRun {
    first: PayloadOffset,
    length: PayloadLength,
    count: ReceiptCount,
}
#[derive(Clone, Debug)]
struct ParkedReceipt {
    layout: Arc<[ReceiptLayout]>,
    session: StagingSession,
    runs: Vec<StagedRun>,
}
/// A finished record, expanded only through ordered delivery or an independent changed-file read.
#[derive(Clone, Debug)]
pub(crate) struct WorkerRecord {
    record: Record,
    parked: Option<ParkedReceipt>,
}
impl From<Record> for WorkerRecord {
    fn from(record: Record) -> Self {
        Self {
            record,
            parked: None,
        }
    }
}
fn handed(record: &mut Record) -> Option<&mut Handed> {
    let FileOutcome::Read(ran) = &mut record.outcome else {
        return None;
    };
    let Handing::Made(handed) = &mut ran.handed else {
        return None;
    };
    Some(handed)
}
impl ParkedReceipt {
    fn freeze(
        buckets: &[HandedWords],
        previous: &mut Option<Arc<[ReceiptLayout]>>,
    ) -> Option<Self> {
        if buckets.is_empty() {
            return None;
        }
        let RecordWords::Staged(first) = &buckets[0].handed else {
            return None;
        };
        let session = first.session;
        if let Some(layout) = previous.as_ref() {
            if buckets.len() != layout.len() {
                return None;
            }
        }
        let mut runs: Vec<StagedRun> = Vec::new();
        let mut next = None;
        for (i, bucket) in buckets.iter().enumerate() {
            if bucket.proof.coverage != FrontierCoverage::Complete {
                return None;
            }
            if let Some(layout) = previous.as_ref() {
                let found = &layout[i];
                if found.0 != bucket.bucket || found.1 != bucket.proof {
                    return None;
                }
            }
            let RecordWords::Staged(words) = &bucket.handed else {
                return None;
            };
            if words.session != session {
                return None;
            }
            let end = words.offset.0.checked_add(words.length.0)?;
            if let Some(run) = runs.last_mut()
                && run.length == words.length
                && next == Some(words.offset.0)
            {
                run.count.0 += 1;
            } else {
                runs.push(StagedRun {
                    first: words.offset,
                    length: words.length,
                    count: ReceiptCount(1),
                });
            }
            next = Some(end);
        }
        let layout = previous.get_or_insert_with(|| {
            buckets
                .iter()
                .map(|b| ReceiptLayout(b.bucket, b.proof.clone()))
                .collect()
        });
        Some(Self {
            layout: Arc::clone(layout),
            session,
            runs,
        })
    }
    fn restored(&self) -> Vec<HandedWords> {
        let mut layout = self.layout.iter();
        let mut buckets = Vec::with_capacity(self.layout.len());
        for run in &self.runs {
            let mut offset = run.first;
            for _ in 0..run.count.0 {
                let Some(expected) = layout.next() else {
                    unreachable!()
                };
                buckets.push(HandedWords {
                    bucket: expected.0,
                    proof: expected.1.clone(),
                    handed: RecordWords::Staged(StagedWords {
                        session: self.session,
                        offset,
                        length: run.length,
                    }),
                });
                offset = PayloadOffset(offset.0 + run.length.0);
            }
        }
        debug_assert!(layout.next().is_none());
        buckets
    }
}

impl WorkerRecord {
    /// Release complete staged descriptors after the original file outcome, proof, faults and staging have been decided.
    pub(crate) fn park(&mut self, layout: &mut Option<Arc<[ReceiptLayout]>>) {
        let Some(handed) = handed(&mut self.record) else {
            return;
        };
        if !handed.partials.is_empty() {
            return;
        }
        let Some(view) = ParkedReceipt::freeze(&handed.buckets, layout) else {
            return;
        };
        handed.buckets = Vec::new();
        self.parked = Some(view);
    }
    /// Return all staged descriptors in their original order. A worker record exposes its complete record only here.
    pub(crate) fn into_record(mut self) -> Record {
        if let Some(view) = self.parked {
            let Some(handed) = handed(&mut self.record) else {
                unreachable!()
            };
            handed.buckets = view.restored();
        }
        self.record
    }
}
