//! The prepared programs, the replay and the gating checks the cases of `file_analysis` share.
use super::errors::FileAnalysisError;
use super::service::FileAnalysis;
use super::types::RequestVisitSetting;
use crate::domain::workers::service::tables_for;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::metadata::interface::PlacedScopes;
use ryo_execution::inbound::use_cases::metadata::interface::ScopePlacement;
use ryo_execution::inbound::use_cases::prepare::interface::ObservationIdentity;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution::inbound::use_cases::prepare::types::PlanSource;
use ryo_execution::inbound::use_cases::prepare::types::PrepareRequest;
use ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use ryo_execution::outbound::matching::interface::MatchingPort;
use ryo_execution_runtime::outbound::facts::fixtures::fake::FactsFake;
use ryo_execution_runtime::outbound::facts::interface::FactsPort;
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::types::Relation;
use ryo_relations::domain::relations::types::WorkerNumber;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::types::Emit;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::rows::types::RowCount;
use ryo_values::domain::rows::types::Table;
use ryo_values::domain::runs::types::RuleApplicability;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::stores::types::BucketsWanted;
use std::sync::Arc;
pub(crate) mod capture;
pub(crate) mod failure;
pub(crate) mod gating;
pub(crate) mod manifest;
pub(crate) mod resident;
pub(crate) mod visit;
pub(crate) fn prepared_test_program(
    plans: &[Plan],
    ids: &[RuleId],
    corpus: &[ryo_values::domain::files::types::CorpusInput],
) -> (
    Arc<ryo_execution::inbound::use_cases::execution::interface::PreparedExecutionProgram>,
    ryo_execution::inbound::use_cases::execution::interface::RequestExecution,
) {
    use ryo_values::domain::{
        query_tokens::types::{QueryBundle, QueryDocumentSource, QuerySourceId},
        rule_requests::types::{RuleParameters, RuleSpec},
        stored_facts::types::Digest,
    };
    let engine = PrepareUseCaseImpl::new(
        Arc::new(
            ryo_execution_runtime::outbound::operators::factory::use_case(
                ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(
                    std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case()),
                ),
            ),
        ),
        ryo_execution::derived(),
        Arc::new(ryo_execution_runtime::outbound::digest::factory::blake3()),
    );
    let program = Arc::new(
        engine
            .prepare(PrepareRequest {
                plans: PlanSource::Authored {
                    plans: plans.to_vec(),
                    bindings: vec![Vec::new(); plans.len()],
                },
                rules: ids
                    .iter()
                    .map(|id| RuleSpec {
                        id: id.clone(),
                        query: QueryBundle {
                            root: QueryDocumentSource {
                                id: QuerySourceId(id.0.clone()),
                                text: "".into(),
                            },
                            modules: Default::default(),
                        },
                        applicability: RuleApplicability::Always,
                        given: Vec::new(),
                        observations: Vec::new(),
                        spelled: RuleParameters("".into()),
                        include: Vec::new(),
                        exclude: Vec::new(),
                    })
                    .collect(),
                digests: vec![Digest([0; 16]); plans.len()],
                hosted: Vec::new(),
                observations: ObservationIdentity::Independent,
            })
            .expect("valid prepared test request"),
    );
    let request = ExecutionUseCaseImpl::new().execution(&program, corpus, None.into());
    (program, request)
}

/// A plan of one rule that captures one frontier input.
pub(crate) fn one_capture_plan() -> Plan {
    Plan {
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
        frontier: vec![ryo_values::domain::plan_parts::types::Input::Step(StepId(
            0,
        ))],
    }
}

/// Replays one handed bucket, kept under `proof`, into a worker of the program of `one_capture_plan`, when the run wants `wanted`.
pub(crate) fn replay_under(
    proof: ryo_values::domain::records::types::FrontierProof,
    wanted: &BucketsWanted,
) -> Result<(), FileAnalysisError> {
    replay_under_capture(proof, wanted, true)
}

/// As `replay_under`, with the prepared root's capture flag of the bucket's frontier set to `captured`.
pub(crate) fn replay_under_capture(
    proof: ryo_values::domain::records::types::FrontierProof,
    wanted: &BucketsWanted,
    captured: bool,
) -> Result<(), FileAnalysisError> {
    use super::types::{FileVisit, Worker};
    use crate::domain::visit::types::{Keeping, Noting, RoleRead};
    use crate::domain::workers::types::FileOrder;
    use ryo_relations::domain::relations::types::{CellKind, FileId, Integer, Value, Words};
    use ryo_values::domain::files::types::{FileNumber, FileSize, Nanoseconds};
    use ryo_values::domain::records::types::{
        FileOutcome, Handed, HandedWords, Handing, Ran, Record,
    };
    use ryo_values::domain::row_texts::types::FindingRows;
    use ryo_values::domain::stored_facts::types::{ArenaBytes, Bucket, Digest};
    use ryo_values::domain::table_words::service::{encode_table, pack};
    use ryo_values::domain::table_words::types::WordSlice;
    let mut words = Words::default();
    encode_table(
        &Table {
            kinds: vec![CellKind::Number],
            cells: vec![Value::Number(Integer(7))],
            count: RowCount(1),
        },
        &[],
        &mut words,
    );
    let record = Record {
        executable: Default::default(),
        path: FilePath("case.py".into()),
        size: FileSize(0),
        modified: Nanoseconds(0),
        digest: Digest([3; 16]),
        number: FileNumber(0),
        outcome: FileOutcome::Read(Ran {
            rows: FindingRows::default(),
            number: FileNumber(0),
            interface: Digest([1; 16]),
            noted: Digest([2; 16]),
            frontier: vec![],
            handed: Handing::Made(Handed {
                buckets: vec![HandedWords {
                    bucket: Bucket(0),
                    proof,
                    handed: pack(WordSlice(&words.0)).into(),
                }],
                text: ArenaBytes::default(),
                ..Handed::default()
            }),
        }),
    };
    let plans = [one_capture_plan()];
    let ids = [RuleId("capture".into())];
    let mut metadata = ryo_values::domain::run_metadata::service::from_plans(&plans);
    for rule in &mut metadata {
        rule.frontier_capture.fill(if captured {
            ryo_values::domain::run_metadata::types::FrontierFeed::Captured
        } else {
            ryo_values::domain::run_metadata::types::FrontierFeed::CrossFile
        });
    }
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
                reuse: &ryo_values::domain::stores::types::Reuse::Cross(
                    ryo_values::domain::stores::types::MadeAgain::All,
                ),
                stored: &NOTHING_STORED,
                handed: &NOTHING_HANDED,
                wanted_buckets: wanted,
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
    let mut worker = Worker {
        tables: tables_for(WorkerNumber(0)),
        facts: FactsFake::default().worker().expect("worker"),
        old_rows: None,
        failure: None,
    };
    FileAnalysis::hand_again(&mut worker, &visit, FileOrder(0), FileId(0), &record)
}

/// What the last run kept, for a run that uses nothing of it.
pub(crate) static NOTHING_STORED: std::sync::LazyLock<ryo_values::domain::stores::types::Stored> =
    std::sync::LazyLock::new(ryo_values::domain::stores::types::Stored::default);

/// What the files of the last run handed, for a run that uses nothing of it.
pub(crate) static NOTHING_HANDED: std::sync::LazyLock<
    ryo_values::domain::stores::types::ReadHanded,
> = std::sync::LazyLock::new(ryo_values::domain::stores::types::ReadHanded::default);

/// No bucket of what the files handed, for a run that reads none.
pub(crate) static NOTHING_WANTED: ryo_values::domain::stores::types::BucketsWanted =
    ryo_values::domain::stores::types::BucketsWanted::These(Vec::new());
