//! The cases of read-failure demand across runs and across rules.
use super::super::errors::FileAnalysisError;
use super::visit::visited_inputs;
use super::visit::visited_inputs_with_rule_demands;
use crate::domain::render::types::Answering;
use crate::domain::visit::types::Keeping;
use crate::outbound::corpus::errors::CorpusError;
use crate::outbound::corpus::interface::CorpusPort;
use crate::outbound::corpus::types::CorpusInventory;
use crate::outbound::corpus::types::PathStat;
use crate::outbound::language::fixtures::fake::LanguageFake;
use ryo_execution::inbound::use_cases::metadata::interface::Mask;
use ryo_execution::inbound::use_cases::metadata::interface::Needs;
use ryo_execution::inbound::use_cases::metadata::interface::RelationRead;
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use ryo_execution_runtime::outbound::digest::fixtures::fake::DigestFake;
use ryo_execution_runtime::outbound::facts::fixtures::fake::FactsFake;
use ryo_execution_runtime::outbound::facts::interface::FactsPort;
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::files::types::CorpusEntry;
use ryo_values::domain::files::types::CorpusInput;
use ryo_values::domain::files::types::FileBytes;
use ryo_values::domain::files::types::FileNumber;
use ryo_values::domain::files::types::FileSize;
use ryo_values::domain::files::types::Nanoseconds;
use ryo_values::domain::records::types::FileOutcome;
use ryo_values::domain::records::types::Record;
use ryo_values::domain::records::types::Unreadable;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::stored_facts::types::Digest;
use ryo_values::domain::stored_facts::types::ReadFailure;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
pub(crate) fn read_failure_demand_retries_unchanged_metadata_and_clears_reused_worker_bytes()
-> Result<usize, FileAnalysisError> {
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

pub(crate) fn read_failure_demand_is_limited_to_governing_rules() -> Result<usize, FileAnalysisError>
{
    use crate::domain::reading::service::read_failure_demanded;
    use ryo_execution::inbound::use_cases::metadata::interface::{Needs, RelationRead};
    let wanted = [Relation::FilesystemReadFailure];
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
    assert_eq!(
        read_failure_demanded(
            &MetadataUseCaseImpl,
            &wanted,
            &Mask(vec![1]),
            &needs,
            crate::domain::reading::types::RunRuleCount(2)
        ),
        crate::domain::reading::types::ReadFailureDemand::NotDemanded
    );
    assert_eq!(
        read_failure_demanded(
            &MetadataUseCaseImpl,
            &wanted,
            &Mask(vec![2]),
            &needs,
            crate::domain::reading::types::RunRuleCount(2)
        ),
        crate::domain::reading::types::ReadFailureDemand::Demanded
    );
    assert_eq!(
        read_failure_demanded(
            &MetadataUseCaseImpl,
            &wanted,
            &Mask(vec![1]),
            &[],
            crate::domain::reading::types::RunRuleCount(2)
        ),
        crate::domain::reading::types::ReadFailureDemand::Demanded
    );
    Ok(1)
}

pub(crate) struct ChangingRead {
    reads: AtomicUsize,
    fail_at: usize,
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
