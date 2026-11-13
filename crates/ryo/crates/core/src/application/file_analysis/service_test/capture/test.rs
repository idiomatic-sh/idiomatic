//! The cases of captured frontier buckets and the staging of fresh outcomes.

use crate::application::file_analysis::errors::FileAnalysisError;
use crate::application::file_analysis::fixtures::NOTHING_HANDED;
use crate::application::file_analysis::fixtures::NOTHING_STORED;
use crate::application::file_analysis::fixtures::NOTHING_WANTED;
use crate::application::file_analysis::fixtures::capture::StagingRead;
use crate::application::file_analysis::fixtures::prepared_test_program;
use crate::application::file_analysis::service::FileAnalysis;
use crate::application::file_analysis::types::RequestVisitSetting;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::workers::service::tables_for;
use crate::outbound::corpus::fixtures::fake::CorpusFake;
use crate::outbound::language::fixtures::fake::LanguageFake;
use crate::outbound::lint_store::errors::LintStoreError;
use crate::outbound::memory::fixtures::fake::MemoryFake;
use ryo_execution::inbound::use_cases::metadata::interface::PlacedScopes;
use ryo_execution::inbound::use_cases::metadata::interface::ScopePlacement;
use ryo_execution::outbound::matching::interface::MatchingPort;
use ryo_execution_runtime::outbound::facts::fixtures::fake::FactsFake;
use ryo_execution_runtime::outbound::facts::interface::FactsPort;
use ryo_relations::domain::relations::types::Relation;
use ryo_relations::domain::relations::types::WorkerNumber;
use ryo_values::domain::files::types::FileNumber;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::types::Emit;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::records::types::FileOutcome;
use ryo_values::domain::records::types::Handed;
use ryo_values::domain::records::types::HandedWords;
use ryo_values::domain::records::types::Handing;
use ryo_values::domain::records::types::Ran;
use ryo_values::domain::records::types::RecordWords;
use ryo_values::domain::records::types::StagedWords;
use ryo_values::domain::row_texts::types::FindingRows;
use ryo_values::domain::rows::types::RowCount;
use ryo_values::domain::rows::types::Table;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stored_facts::types::Digest;
use ryo_values::domain::stored_facts::types::PackedWords;
use std::sync::Arc;
use std::sync::atomic::Ordering;

fn a_bad_last_captured_bucket_calls_no_digest_even_after_a_valid_rule_bucket()
-> Result<String, FileAnalysisError> {
    let mut seen: Vec<String> = Vec::new();
    use crate::application::file_analysis::types::{FileVisit, Worker};

    use crate::domain::visit::types::{Keeping, Noting, RoleRead};
    use ryo_execution_runtime::outbound::digest::fixtures::fake::DigestCounting;
    use ryo_relations::domain::relations::types::{
        CellKind, FileId, Integer, Name, NamePlace, Offset, Origin, PlaceNumber, TextHash, Value,
        Words,
    };
    use ryo_values::domain::records::types::Kept;
    use ryo_values::domain::stored_facts::types::{ArenaBase, KeptWords};
    use ryo_values::domain::table_words::service::{encode_table, pack};
    use ryo_values::domain::table_words::types::WordSlice;
    let plan = Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: vec![],
        steps: vec![ryo_values::domain::plans::types::PlanStep {
            scope: ryo_values::domain::plan_parts::types::StepScope::PerFile,
            operation: ryo_values::domain::plans::types::Operation::Select {
                from: ryo_values::domain::plan_parts::types::Input::Relation(Relation::Function),
                comparisons: Vec::new(),
                kept: vec![ColumnIndex(0)],
            },
        }],
        comparison_origins: vec![],
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        frontier: vec![
            ryo_values::domain::plan_parts::types::Input::Step(StepId(0)),
            ryo_values::domain::plan_parts::types::Input::Step(StepId(0)),
        ],
    };
    let plans = [plan];
    let ids = [RuleId("capture".into())];
    let metadata = ryo_values::domain::run_metadata::service::from_plans(&plans);
    let (program, request) = prepared_test_program(&plans, &ids, &[]);
    let placed = PlacedScopes {
        placement: ScopePlacement::default(),
        matcher: ryo_execution::outbound::matching::fixtures::fake::MatchingFake
            .compile(&[])
            .expect("matcher"),
    };
    let visit = FileVisit {
        store: None,
        setting: RequestVisitSetting {
            source_language: None,
            program,
            services: &request,
            domain: crate::domain::workers::types::VisitSetting {
                metadata: &metadata,
                wanted: &[],
                paths: &[],
                first_bucket: &[Bucket(0)],
                ids: &ids,
                applicability: &[],
                applicability_can_gate: crate::domain::workers::types::Gating::MayGate,
                placed: &placed,
                records: &[],
                reuse: &ryo_values::domain::stores::types::Reuse::Whole,
                stored: &NOTHING_STORED,
                handed: &NOTHING_HANDED,
                wanted_buckets: &NOTHING_WANTED,
                budget: ryo_values::domain::runs::types::MemoryBytes(u64::MAX),
                noted: Noting::Unwanted,
                roled: RoleRead::Skips,
                needs: &[],
                answering: &crate::domain::render::types::Answering::Findings,
                keeping: Keeping::Keeps,
                unknown: &[],
            },
            #[cfg(feature = "fused-evaluation")]
            fused: None,
        },
    };
    let digest = Arc::new(DigestCounting::default());
    let analysis = FileAnalysis::new(
        Arc::new(CorpusFake::default()),
        Arc::new(LanguageFake::default()),
        digest.clone(),
        Arc::new(MemoryFake::default()),
    );
    let mut worker = Worker {
        tables: tables_for(WorkerNumber(0)),
        facts: FactsFake::default().worker().expect("facts worker"),
        old_rows: None,
        failure: None,
    };
    let packed = |value, places: &[NamePlace]| {
        let mut words = Words::default();
        let kind = if matches!(value, Value::File(_)) {
            CellKind::File
        } else if matches!(value, Value::Name(_)) {
            CellKind::Symbol
        } else {
            CellKind::Number
        };
        encode_table(
            &Table {
                kinds: vec![kind],
                cells: vec![value],
                count: RowCount(1),
            },
            places,
            &mut words,
        );
        KeptWords(pack(WordSlice(&words.0)))
    };
    let proof = |input| ryo_values::domain::records::types::FrontierProof {
        rule: ryo_values::domain::records::types::ProducerRule(0),
        input: ryo_values::domain::records::types::FrontierOrdinal(input),
        rule_digest: ryo_values::domain::stored_facts::types::Digest([0; 16]),
        coverage: ryo_values::domain::records::types::FrontierCoverage::Complete,
    };
    let good = Kept {
        bucket: Bucket(0),
        proof: proof(0),
        words: packed(Value::Number(Integer(7)), &[]),
    };
    for (value, places) in [
        (Value::File(FileId(99)), vec![]),
        (
            Value::Name(Name {
                hash: TextHash(1),
                place: PlaceNumber(0),
            }),
            vec![NamePlace {
                origin: Origin::Derived(WorkerNumber(99)),
                file: FileId(0),
                start: Offset(9),
                end: Offset(12),
            }],
        ),
        (
            Value::Name(Name {
                hash: TextHash(1),
                place: PlaceNumber(0),
            }),
            vec![NamePlace {
                origin: Origin::Derived(WorkerNumber(0)),
                file: FileId(0),
                start: Offset(2),
                end: Offset(4),
            }],
        ),
    ] {
        worker.tables.captured = vec![
            good.clone(),
            Kept {
                bucket: Bucket(1),
                proof: proof(1),
                words: packed(value, &places),
            },
        ];
        seen.push(format!(
            "{:?}",
            analysis
                .recorded(&worker, &visit, FileId(0), ArenaBase(7))
                .is_none()
        ));
        seen.push(format!("{:?}", digest.digests.load(Ordering::Relaxed)));
    }
    worker.tables.captured = vec![good];
    seen.push(format!(
        "{:?}",
        analysis
            .recorded(&worker, &visit, FileId(0), ArenaBase(7))
            .is_some()
    ));
    seen.push(format!("{:?}", digest.digests.load(Ordering::Relaxed)));
    Ok(seen.join("|"))
}

/// Only post-normalization outcomes can reach the staging boundary; IO failures remain failures.
fn successful_outcomes_stage_once_and_keep_digests_without_retaining_payloads()
-> Result<String, FileAnalysisError> {
    let mut seen: Vec<String> = Vec::new();
    let outcome = FileOutcome::Read(Ran {
        rows: FindingRows::default(),
        number: FileNumber(0),
        interface: Digest([1; 16]),
        noted: Digest([2; 16]),
        frontier: vec![],
        handed: Handing::Made(Handed {
            buckets: (0..64)
                .map(|bucket| HandedWords {
                    bucket: Bucket(bucket),
                    proof: crate::outbound::lint_store::fixtures::proof(),
                    handed: PackedWords(vec![7; 4096]).into(),
                })
                .collect(),
            ..Handed::default()
        }),
    });
    let read = StagingRead::default();
    seen.push(format!(
        "{:?}",
        FileAnalysis::stage_outcome(Some(&read), None)
            .expect("no outcome")
            .is_none()
    ));
    seen.push(format!("{:?}", read.calls.load(Ordering::Relaxed)));
    seen.push(format!(
        "{:?}",
        (FileAnalysis::stage_outcome(None, Some(outcome.clone())).expect("no store"))
            == (Some(outcome.clone()))
    ));
    let Some(FileOutcome::Read(ran)) =
        FileAnalysis::stage_outcome(Some(&read), Some(outcome.clone())).expect("stage")
    else {
        panic!("read")
    };
    seen.push(format!("{:?}", (ran.interface) == (Digest([1; 16]))));
    seen.push(format!("{:?}", (ran.noted) == (Digest([2; 16]))));
    let Handing::Made(handed) = ran.handed else {
        panic!("fresh outcome")
    };
    seen.push(format!("{:?}", read.calls.load(Ordering::Relaxed)));
    seen.push(format!(
        "{:?}",
        handed.buckets.iter().all(|row| matches!(
            row.handed,
            RecordWords::Staged(StagedWords {
                length: ryo_values::domain::records::types::PayloadLength(4096),
                ..
            })
        ))
    ));
    let failing = StagingRead {
        fail: true,
        ..StagingRead::default()
    };
    seen.push(format!(
        "{:?}",
        matches!(
            FileAnalysis::stage_outcome(Some(&failing), Some(outcome)),
            Err(FileAnalysisError::Store(LintStoreError::NothingRead))
        )
    ));
    seen.push(format!("{:?}", failing.calls.load(Ordering::Relaxed)));
    Ok(seen.join("|"))
}

/// Which scenario a row runs.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The `a_bad_last_captured_bucket_calls_no_digest_even_after_a_valid_rule_bucket` scenario.
    ABadLastCapturedBucketCallsNoDigestEvenAfterAValidRuleBucket,
    /// The `successful_outcomes_stage_once_and_keep_digests_without_retaining_payloads` scenario.
    SuccessfulOutcomesStageOnceAndKeepDigestsWithoutRetainingPayloads,
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
        name: CaseName("a_bad_last_captured_bucket_calls_no_digest_even_after_a_valid_rule_bucket"),
        word: Word::Succeeds,
        input: || Given::ABadLastCapturedBucketCallsNoDigestEvenAfterAValidRuleBucket,
        expect: Expect::Value("true|0|true|0|true|0|true|2"),
    },
    Case {
        name: CaseName(
            "successful_outcomes_stage_once_and_keep_digests_without_retaining_payloads",
        ),
        word: Word::Succeeds,
        input: || Given::SuccessfulOutcomesStageOnceAndKeepDigestsWithoutRetainingPayloads,
        expect: Expect::Value("true|0|true|true|true|64|true|true|1"),
    },
];

/// Runs the scenario the row names.
fn subject(given: Input) -> Result<Value, Failure> {
    let observed = match given {
        Given::ABadLastCapturedBucketCallsNoDigestEvenAfterAValidRuleBucket => {
            a_bad_last_captured_bucket_calls_no_digest_even_after_a_valid_rule_bucket()?
        }
        Given::SuccessfulOutcomesStageOnceAndKeepDigestsWithoutRetainingPayloads => {
            successful_outcomes_stage_once_and_keep_digests_without_retaining_payloads()?
        }
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
