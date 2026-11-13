//! The cases of read-failure demand across runs and across rules.

use crate::application::file_analysis::errors::FileAnalysisError;
use crate::application::file_analysis::fixtures::failure::ChangingRead;
use crate::application::file_analysis::fixtures::visit::visited_inputs;
use crate::application::file_analysis::fixtures::visit::visited_inputs_with_rule_demands;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::render::types::Answering;
use crate::domain::visit::types::Keeping;
use crate::outbound::language::fixtures::fake::LanguageFake;
use ryo_execution::inbound::use_cases::metadata::interface::Mask;
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use ryo_execution_runtime::outbound::digest::fixtures::fake::DigestFake;
use ryo_execution_runtime::outbound::facts::fixtures::fake::FactsFake;
use ryo_execution_runtime::outbound::facts::interface::FactsPort;
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::files::types::CorpusEntry;
use ryo_values::domain::files::types::CorpusInput;
use ryo_values::domain::files::types::FileNumber;
use ryo_values::domain::files::types::FileSize;
use ryo_values::domain::files::types::Nanoseconds;
use ryo_values::domain::records::types::FileOutcome;
use ryo_values::domain::records::types::Record;
use ryo_values::domain::records::types::Unreadable;
use ryo_values::domain::stored_facts::types::Digest;
use ryo_values::domain::stored_facts::types::ReadFailure;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

fn read_failure_demand_retries_unchanged_metadata_and_clears_reused_worker_bytes()
-> Result<String, FileAnalysisError> {
    let mut seen: Vec<String> = Vec::new();
    for fail_at in [0, 1, 2] {
        let corpus = Arc::new(ChangingRead {
            reads: AtomicUsize::new(0),
            fail_at,
            dirty_starts: AtomicUsize::new(0),
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
        seen.push(format!("{:?}", worker.failure.is_none()));
        seen.push(format!("{:?}", corpus.reads.load(Ordering::Relaxed)));
        seen.push(format!("{:?}", corpus.dirty_starts.load(Ordering::Relaxed)));
        seen.push(format!("{:?}", language.parses.load(Ordering::Relaxed)));
        if fail_at == 2 {
            seen.push(format!("{:?}", worker.tables.bytes.0.is_empty()));
            seen.push(format!(
                "{:?}",
                worker
                    .tables
                    .partials
                    .iter()
                    .any(|partial| partial.relation == Relation::FilesystemLineCount)
            ));
            seen.push(format!(
                "{:?}",
                worker
                    .tables
                    .partials
                    .iter()
                    .any(|partial| partial.relation == Relation::Function)
            ));
            seen.push(format!(
                "{:?}",
                !worker
                    .tables
                    .partials
                    .iter()
                    .any(|partial| partial.relation == Relation::FilesystemReadFailure)
            ));
        } else {
            seen.push(format!(
                "{:?}",
                (worker.tables.bytes.0) == (b"fn main() {}")
            ));
        }
        seen.push(format!("{:?}", worker.tables.unread.len()));
        seen.push(format!("{:?}", worker.tables.unrecorded));
        seen.push(format!("{:?}", worker.tables.fresh.len()));
        if fail_at == 0 {
            use ryo_execution::inbound::use_cases::metadata::interface::{Needs, RelationRead};
            use ryo_values::domain::runs::types::RuleId;
            let excluded = Arc::new(ChangingRead {
                reads: AtomicUsize::new(0),
                fail_at: usize::MAX,
                dirty_starts: AtomicUsize::new(0),
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
            seen.push(format!("{:?}", replayed.failure.is_none()));
            seen.push(format!("{:?}", excluded.reads.load(Ordering::Relaxed)));
            seen.push(format!(
                "{:?}",
                excluded.dirty_starts.load(Ordering::Relaxed)
            ));

            let included = Arc::new(ChangingRead {
                reads: AtomicUsize::new(0),
                fail_at: usize::MAX,
                dirty_starts: AtomicUsize::new(0),
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
            seen.push(format!("{:?}", fresh.failure.is_none()));
            seen.push(format!("{:?}", included.reads.load(Ordering::Relaxed)));
            seen.push(format!(
                "{:?}",
                included.dirty_starts.load(Ordering::Relaxed)
            ));
        }
    }
    Ok(seen.join("|"))
}

fn read_failure_demand_is_limited_to_governing_rules() -> Result<String, FileAnalysisError> {
    let mut seen: Vec<String> = Vec::new();
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
    seen.push(format!(
        "{:?}",
        (read_failure_demanded(
            &MetadataUseCaseImpl,
            &wanted,
            &Mask(vec![1]),
            &needs,
            crate::domain::reading::types::RunRuleCount(2)
        )) == (crate::domain::reading::types::ReadFailureDemand::NotDemanded)
    ));
    seen.push(format!(
        "{:?}",
        (read_failure_demanded(
            &MetadataUseCaseImpl,
            &wanted,
            &Mask(vec![2]),
            &needs,
            crate::domain::reading::types::RunRuleCount(2)
        )) == (crate::domain::reading::types::ReadFailureDemand::Demanded)
    ));
    seen.push(format!(
        "{:?}",
        (read_failure_demanded(
            &MetadataUseCaseImpl,
            &wanted,
            &Mask(vec![1]),
            &[],
            crate::domain::reading::types::RunRuleCount(2)
        )) == (crate::domain::reading::types::ReadFailureDemand::Demanded)
    ));
    Ok(seen.join("|"))
}

/// The relations a failed file is read in part, run with engine rows asked: what the run settled, which the visit keeps.
fn a_running_failed_file_s_partiality_is_the_run_s() -> Result<String, FileAnalysisError> {
    let corpus = Arc::new(ChangingRead {
        reads: AtomicUsize::new(0),
        fail_at: 0,
        dirty_starts: AtomicUsize::new(0),
    });
    let entry = CorpusEntry {
        executable: Default::default(),
        path: FilePath("failed.rs".into()),
        size: FileSize(12),
        modified: Nanoseconds(42),
    };
    let (worker, _) = visited_inputs(
        corpus,
        &[CorpusInput::Available(entry)],
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
        None,
        1,
    )?;
    Ok(worker
        .tables
        .partials
        .iter()
        .map(|partial| format!("{:?} {}", partial.relation, partial.cause.0))
        .collect::<Vec<_>>()
        .join("|"))
}

/// Which scenario a row runs.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The `read_failure_demand_retries_unchanged_metadata_and_clears_reused_worker_bytes` scenario.
    ReadFailureDemandRetriesUnchangedMetadataAndClearsReusedWorkerBytes,
    /// The `read_failure_demand_is_limited_to_governing_rules` scenario.
    ReadFailureDemandIsLimitedToGoverningRules,
    /// The `a_running_failed_file_s_partiality_is_the_run_s` scenario.
    ARunningFailedFileSPartialityIsTheRunS,
}
/// What a row is given.
type Input = Given;
/// What the scenario observed, each check in order, joined by `|`.
type Value = &'static str;
/// What a scenario's run refuses.
type Failure = FileAnalysisError;

/// The rows of this group's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "read_failure_demand_retries_unchanged_metadata_and_clears_reused_worker_bytes",
        ),
        word: Word::Succeeds,
        input: || Given::ReadFailureDemandRetriesUnchangedMetadataAndClearsReusedWorkerBytes,
        expect: Expect::Value(
            "true|3|0|2|true|1|[FilePath(\"same.rs\")]|2|true|0|0|true|1|0|true|3|0|2|true|1|[FilePath(\"same.rs\")]|2|true|3|0|2|true|true|true|true|1|[FilePath(\"same.rs\")]|2",
        ),
    },
    Case {
        name: CaseName("read_failure_demand_is_limited_to_governing_rules"),
        word: Word::Succeeds,
        input: || Given::ReadFailureDemandIsLimitedToGoverningRules,
        expect: Expect::Value("true|true|true"),
    },
    Case {
        name: CaseName("a_running_failed_file_s_partiality_is_the_run_s"),
        word: Word::Succeeds,
        input: || Given::ARunningFailedFileSPartialityIsTheRunS,
        expect: Expect::Value(
            "Function cannot read `failed.rs`: read denied|FilesystemLineCount cannot read `failed.rs`: read denied",
        ),
    },
];

/// Runs the scenario the row names.
fn subject(given: Input) -> Result<Value, Failure> {
    let observed = match given {
        Given::ReadFailureDemandRetriesUnchangedMetadataAndClearsReusedWorkerBytes => {
            read_failure_demand_retries_unchanged_metadata_and_clears_reused_worker_bytes()?
        }
        Given::ReadFailureDemandIsLimitedToGoverningRules => {
            read_failure_demand_is_limited_to_governing_rules()?
        }
        Given::ARunningFailedFileSPartialityIsTheRunS => {
            a_running_failed_file_s_partiality_is_the_run_s()?
        }
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
