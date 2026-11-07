//! The prepared programs, the replay and the gating checks the cases of `file_analysis` share.
use crate::domain::visit::types::Keeping;
use crate::domain::workers::service::tables_for;
use crate::domain::workers::types::Gating;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::metadata::interface::Mask;
use ryo_execution::inbound::use_cases::metadata::interface::PlacedScopes;
use ryo_execution::inbound::use_cases::metadata::interface::ScopePlacement;
use ryo_execution::inbound::use_cases::prepare::interface::ObservationIdentity;
use ryo_execution::inbound::use_cases::prepare::types::PlanSource;
use ryo_execution::inbound::use_cases::prepare::types::PrepareRequest;
use ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use ryo_execution::outbound::facts::fixtures::fake::FactsFake;
use ryo_language::domain::elements::types::SyntaxAcceptance;
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::types::Relation;
use ryo_relations::domain::relations::types::WorkerNumber;
use ryo_values::domain::files::types::FileNumber;
use ryo_values::domain::files::types::FileSize;
use ryo_values::domain::files::types::Nanoseconds;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::types::Emit;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::records::types::FileOutcome;
use ryo_values::domain::records::types::Handed;
use ryo_values::domain::records::types::HandedWords;
use ryo_values::domain::records::types::Handing;
use ryo_values::domain::records::types::Ran;
use ryo_values::domain::records::types::Record;
use ryo_values::domain::row_texts::types::FindingRows;
use ryo_values::domain::rows::types::RowCount;
use ryo_values::domain::rows::types::Table;
use ryo_values::domain::runs::types::RuleApplicability;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stored_facts::types::Digest;
use ryo_values::domain::stores::types::BucketsWanted;
use std::sync::Arc;
use super::super::errors::FileAnalysisError;
use super::super::service::FileAnalysis;
use super::super::types::RequestVisitSetting;
use super::super::types::RuleGate;
pub(super) fn prepared_test_program(
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
            ryo_execution::outbound::operators::factory::use_case(ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case()))),
        ),
        Arc::new(ryo_execution::outbound::derived::factory::semantics()),
    );
    let program = Arc::new(
        engine
            .prepare(PrepareRequest { plans: PlanSource::Authored { plans: plans.to_vec(), bindings: vec![Vec::new(); plans.len()] }, rules: ids
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
                    .collect(), digests: vec![Digest([0; 16]); plans.len()], hosted: Vec::new(), observations: ObservationIdentity::Independent })
            .expect("valid prepared test request"),
    );
    let request = ExecutionUseCaseImpl::new().execution(&program, corpus);
    (program, request)
}

/// A plan of one rule that captures one frontier input.
pub(super) fn one_capture_plan() -> Plan {
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
pub(super) fn replay_under(
    proof: ryo_values::domain::records::types::FrontierProof,
    wanted: &BucketsWanted,
) -> Result<(), FileAnalysisError> {
    replay_under_capture(proof, wanted, true)
}

/// As `replay_under`, with the prepared root's capture flag of the bucket's frontier set to `captured`.
pub(super) fn replay_under_capture(
    proof: ryo_values::domain::records::types::FrontierProof,
    wanted: &BucketsWanted,
    captured: bool,
) -> Result<(), FileAnalysisError> {
    use super::super::types::{FileVisit, Worker};
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
        },
    };
    let mut worker = Worker {
        tables: tables_for(WorkerNumber(0), &[]),
        facts: FactsFake::default().worker().expect("worker"),
        old_rows: None,
        failure: None,
    };
    FileAnalysis::hand_again(&mut worker, &visit, FileOrder(0), FileId(0), &record)
}

/// Check a rejected or undetermined source without mixing its owner tables with other cases.
pub(super) fn assert_gated_acceptance(
    setting: &RequestVisitSetting<'_>,
    acceptance: SyntaxAcceptance,
    expected_reason: &str,
) {
    let mut tables = tables_for(WorkerNumber(0), setting.metadata);
    tables.mask = Mask(vec![0b11]);
    let skipped = FileAnalysis::gate_rules(
        &mut tables,
        setting,
        ryo_relations::domain::relations::types::FileId(0),
        acceptance,
    );
    assert_eq!(skipped, [RuleGate::Blocked, RuleGate::Open]);
    assert_eq!(tables.mask.0, [0b10]);
    assert_eq!(
        FileAnalysis::active_wanted(&tables, setting, acceptance).as_ref(),
        [Relation::Identifier]
    );
    if acceptance == SyntaxAcceptance::Rejected {
        assert!(tables.unknowns.is_empty());
        assert_eq!(tables.skipped.len(), 1);
        assert_eq!(tables.skipped[0].rule, setting.ids[0]);
        assert_eq!(tables.skipped[0].path, setting.paths[0]);
        assert_eq!(
            tables.skipped[0].reason,
            ryo_values::domain::runs::types::SkipReason::SyntaxRejected
        );
        assert!(expected_reason.contains("rejected"));
    } else {
        assert_eq!(tables.unknowns.len(), 1);
        assert_eq!(tables.unknowns[0].rule, setting.ids[0]);
        assert_eq!(tables.unknowns[0].path, setting.paths[0]);
        assert_eq!(&*tables.unknowns[0].message.0, expected_reason);
        assert!(tables.skipped.is_empty());
    }
}

/// Accepted syntax keeps enabled rules; absent applicability metadata still records unknowns.
pub(super) fn assert_accepted_and_missing_applicability(setting: &RequestVisitSetting<'_>) {
    let mut tables = tables_for(WorkerNumber(0), setting.metadata);
    tables.mask = Mask(vec![0b11]);
    assert_eq!(
        FileAnalysis::gate_rules(
            &mut tables,
            setting,
            ryo_relations::domain::relations::types::FileId(0),
            SyntaxAcceptance::Accepted,
        ),
        [RuleGate::Open, RuleGate::Open]
    );
    assert_eq!(tables.mask.0, [0b11]);
    assert_eq!(
        FileAnalysis::active_wanted(&tables, setting, SyntaxAcceptance::Accepted).as_ref(),
        [Relation::Function, Relation::Identifier]
    );
    let always_setting = RequestVisitSetting {
        source_language: None,
        domain: crate::domain::workers::types::VisitSetting {
            metadata: &setting.metadata[1..],
            ids: &setting.ids[1..],
            applicability: &setting.applicability[1..],
            applicability_can_gate: crate::domain::workers::types::Gating::NeverGates,
            needs: &setting.needs[1..],
            ..(setting.clone()).domain
        },
        ..setting.clone()
    };
    assert!(matches!(
        FileAnalysis::active_wanted(&tables, &always_setting, SyntaxAcceptance::Rejected),
        std::borrow::Cow::Borrowed(_)
    ));
    assert_eq!(
        FileAnalysis::may_gate_rules(setting, SyntaxAcceptance::Accepted),
        Gating::NeverGates
    );
    assert_eq!(
        FileAnalysis::may_gate_rules(setting, SyntaxAcceptance::Rejected),
        Gating::MayGate
    );
    assert!(tables.unknowns.is_empty());

    let missing_policy = RequestVisitSetting {
        domain: crate::domain::workers::types::VisitSetting {
            applicability: &[],
            applicability_can_gate: crate::domain::workers::types::Gating::MayGate,
            ..(setting.clone()).domain
        },
        ..setting.clone()
    };
    assert_eq!(
        FileAnalysis::may_gate_rules(&missing_policy, SyntaxAcceptance::Accepted),
        Gating::MayGate
    );
    let mut missing_tables = tables_for(WorkerNumber(0), setting.metadata);
    missing_tables.mask = Mask(vec![0b11]);
    assert_eq!(
        FileAnalysis::gate_rules(
            &mut missing_tables,
            &missing_policy,
            ryo_relations::domain::relations::types::FileId(0),
            SyntaxAcceptance::Accepted,
        ),
        [RuleGate::Blocked, RuleGate::Blocked]
    );
    assert_eq!(missing_tables.mask.0, [0]);
    assert!(
        FileAnalysis::active_wanted(&missing_tables, &missing_policy, SyntaxAcceptance::Accepted)
            .is_empty()
    );
    assert_eq!(missing_tables.unknowns.len(), 2);
    assert!(missing_tables.skipped.is_empty());
    assert!(missing_tables.unknowns.iter().all(|unknown| {
        &*unknown.message.0 == "rule skipped: applicability metadata unavailable"
    }));
}

/// What the last run kept, for a run that uses nothing of it.
pub(super) static NOTHING_STORED: std::sync::LazyLock<ryo_values::domain::stores::types::Stored> =
    std::sync::LazyLock::new(ryo_values::domain::stores::types::Stored::default);

/// What the files of the last run handed, for a run that uses nothing of it.
pub(super) static NOTHING_HANDED: std::sync::LazyLock<ryo_values::domain::stores::types::ReadHanded> =
    std::sync::LazyLock::new(ryo_values::domain::stores::types::ReadHanded::default);

/// No bucket of what the files handed, for a run that reads none.
pub(super) static NOTHING_WANTED: ryo_values::domain::stores::types::BucketsWanted =
    ryo_values::domain::stores::types::BucketsWanted::These(Vec::new());



