//! The scenario `read_failure_demand_retries_unchanged_metadata_and_clears_reused_worker_bytes` of the cases of this file.

use super::*;

pub(super) fn read_failure_demand_retries_unchanged_metadata_and_clears_reused_worker_bytes() -> Result<Value, Failure> {
    use read_failure_demand_retries_unchanged_metadata_and_clears_reused_worker_bytes_doubles::ChangingRead;
    for fail_at in [0, 1, 2] {
        let corpus = Arc::new(ChangingRead {
            reads: AtomicUsize::new(0),
            fail_at,
        });
        let entry = CorpusEntry {
            executable: Default::default(),
            path: FilePath("same.rs".into()),
            size: FileSize(12),
            modified: Nanoseconds(42),
        };
        let kept = Record {
            executable: entry.executable,
            path: entry.path.clone(),
            size: entry.size,
            modified: entry.modified,
            digest: Digest([7; 16]),
            number: FileNumber(0),
            outcome: FileOutcome::Unread {
                reason: Unreadable::Failed(ReadFailure("previous refusal".into())),
                ran: None,
            },
        };
        let (worker, language) = visited_inputs(
            corpus.clone(),
            &[CorpusInput::Available(entry.clone())],
            &[
                Relation::FilesystemReadFailure,
                Relation::FilesystemLineCount,
                Relation::Function,
            ],
            LanguageFake::default(),
            &Answering::Findings,
            Keeping::Keeps,
            Arc::new(DigestFake),
            FactsFake::default().worker().unwrap(),
            Some(&kept),
            3,
        )
        .unwrap();
        assert!(worker.failure.is_none());
        assert_eq!(
            corpus.reads.load(Ordering::Relaxed),
            3,
            "metadata equality cannot certify current readability"
        );
        assert_eq!(
            language.parses.load(Ordering::Relaxed),
            2,
            "a failed disk attempt never reaches the parser"
        );
        if fail_at == 2 {
            assert!(worker.tables.bytes.0.is_empty());
            assert!(
                worker
                    .tables
                    .partials
                    .iter()
                    .any(|partial| partial.relation == Relation::FilesystemLineCount)
            );
            assert!(
                worker
                    .tables
                    .partials
                    .iter()
                    .any(|partial| partial.relation == Relation::Function)
            );
            assert!(
                !worker
                    .tables
                    .partials
                    .iter()
                    .any(|partial| partial.relation == Relation::FilesystemReadFailure)
            );
        } else {
            assert_eq!(worker.tables.bytes.0, b"fn main() {}");
        }
        assert_eq!(worker.tables.unread.len(), 1);
        assert_eq!(
            worker.tables.unrecorded,
            [FilePath("same.rs".into())],
            "the failed visit is retried rather than persisted"
        );
        assert_eq!(
            worker.tables.fresh.len(),
            2,
            "only successfully admitted visits have reusable records"
        );
        if fail_at == 0 {
            use ryo_execution::inbound::use_cases::metadata::interface::{Needs, RelationRead};
            use ryo_values::domain::runs::types::RuleId;
            let excluded = Arc::new(ChangingRead {
                reads: AtomicUsize::new(0),
                fail_at: usize::MAX,
            });
            let needs = [
                Needs::default(),
                Needs {
                    reads: vec![RelationRead {
                        relation: Relation::FilesystemReadFailure,
                        kinds: vec![],
                    }],
                    ..Needs::default()
                },
            ];
            let ids = [RuleId("ordinary".into()), RuleId("read-failure".into())];
            let (replayed, _) = visited_inputs_with_rule_demands(
                excluded.clone(),
                &[CorpusInput::Available(entry.clone())],
                &[Relation::FilesystemReadFailure],
                LanguageFake::default(),
                &Answering::Findings,
                Keeping::Keeps,
                Arc::new(DigestFake),
                FactsFake::default().worker().unwrap(),
                Some(&kept),
                1,
                &ids,
                &needs,
                Mask(vec![1]),
            )
            .unwrap();
            assert!(replayed.failure.is_none());
            assert_eq!(excluded.reads.load(Ordering::Relaxed), 0);

            let included = Arc::new(ChangingRead {
                reads: AtomicUsize::new(0),
                fail_at: usize::MAX,
            });
            let (fresh, _) = visited_inputs_with_rule_demands(
                included.clone(),
                &[CorpusInput::Available(entry.clone())],
                &[Relation::FilesystemReadFailure],
                LanguageFake::default(),
                &Answering::Findings,
                Keeping::Keeps,
                Arc::new(DigestFake),
                FactsFake::default().worker().unwrap(),
                Some(&kept),
                1,
                &ids,
                &needs,
                Mask(vec![2]),
            )
            .unwrap();
            assert!(fresh.failure.is_none());
            assert_eq!(included.reads.load(Ordering::Relaxed), 1);
        }
    }
    Ok(1)
}

mod read_failure_demand_retries_unchanged_metadata_and_clears_reused_worker_bytes_doubles {
    use super::*;
    use crate::domain::render::types::Answering;
    use crate::domain::visit::types::Keeping;
    use crate::outbound::corpus::{
        errors::CorpusError,
        interface::CorpusPort,
        types::{CorpusInventory, PathStat},
    };
    use ryo_values::domain::files::types::{
        CorpusEntry, CorpusInput, FileBytes, FileNumber, FileSize, Nanoseconds,
    };
    use ryo_values::domain::records::types::{FileOutcome, Record, Unreadable};
    use ryo_values::domain::stored_facts::types::{Digest, ReadFailure};
    use std::sync::atomic::AtomicUsize;

    pub(super) struct ChangingRead {
        pub(super) reads: AtomicUsize,
        pub(super) fail_at: usize,
    }

    impl CorpusPort for ChangingRead {
        fn observe_entry(
            &self,
            path: &FilePath,
        ) -> ryo_language::domain::project_context::types::EntryObservation {
            ryo_language::domain::project_context::types::EntryObservation {
                path: path.clone(),
                state: ryo_language::domain::project_context::types::ObservedEntryState::Unknown,
            }
        }
        fn inventory(&self) -> Result<CorpusInventory, CorpusError> {
            unreachable!("explicit test inventory")
        }
        fn stat(&self, _: &FilePath) -> Result<PathStat, CorpusError> {
            unreachable!("no dependency lookup")
        }
        fn read(&self, entry: &CorpusEntry, bytes: &mut FileBytes) -> Result<(), CorpusError> {
            assert!(
                bytes.0.is_empty(),
                "each real attempt starts without earlier source or partial error bytes"
            );
            let at = self.reads.fetch_add(1, Ordering::Relaxed);
            if at == self.fail_at {
                bytes.0.extend_from_slice(b"partial failed bytes");
                Err(CorpusError::Unreadable {
                    path: entry.path.0.to_string(),
                    reason: "read denied".into(),
                })
            } else {
                bytes.0.extend_from_slice(b"fn main() {}");
                Ok(())
            }
        }
    }
}
