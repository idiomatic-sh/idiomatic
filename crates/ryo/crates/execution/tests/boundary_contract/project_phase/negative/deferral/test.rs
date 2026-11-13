//! Deferred roots do not hide owning-phase faults or renumber file frontier refusals.
use crate::boundary_fixture::fixtures::{FileRun, ran, root, run_file};
use crate::boundary_fixture::fixtures::{authored_request, corpus, engine, prepared, source};
use ryo_execution::inbound::use_cases::execution::fixtures::TableFacts;
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::Covered;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::Partials;
use ryo_execution::inbound::use_cases::execution::interface::PhaseRules;
use ryo_execution::inbound::use_cases::execution::interface::ProgramProjectPhasePort;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::interface::RootOutcome;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::interface::{FileStop, RuleAfter};
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase as _;
use ryo_execution_runtime::outbound::facts::fixtures::project_phase_negative::NoFileFrontiers;
use ryo_execution_runtime::outbound::facts::fixtures::project_phase_source_facts::ProjectFacts;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::{
    fixtures::kinds,
    types::{CellKind, FileId, Relation},
};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input, InputNumber, StepId, StepScope},
    plans::types::{Operation, PlanStep},
    rows::errors::RowsError,
    stored_facts::types::Bucket,
};
use std::sync::atomic::Ordering;

#[test]
fn a_file_emit_defers_at_project_scope_but_its_real_file_failure_is_not_deferred() {
    let engine = engine();
    let prepared = prepared(&engine);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None.into());
    let covered = [Covered {
        file: FileId(0),
        rules: vec![RuleIndex(0)],
        partial: vec![],
    }];
    let phase = request.project_phase().unwrap();
    let project = ProjectFacts::new(Relation::ModuleIdentity);
    let mut symbols = Symbols::default();
    let project_symbols = Symbols::default();
    let partials = Partials::default();
    let result = crate::boundary_fixture::fixtures::project_root(
        &phase,
        PhaseRules {
            rules: &[RuleIndex(0)],
            first: &[Bucket(0)],
            corpus: &corpus,
            covered: &covered,
            skipped: &[],
            inapplicable: &[
                ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles::default(
                ),
            ],
            refused: &[
                ryo_execution::inbound::use_cases::execution::interface::RefusedFiles::default(),
            ],
            facts: &NoFileFrontiers,
            project: &project,
            project_symbols: &project_symbols,
            arenas: &[],
        },
        &partials,
        &partials,
        &mut symbols,
    )
    .unwrap();
    assert!(matches!(
        result.emit,
        RootOutcome::Deferred {
            phase: StepScope::PerFile
        }
    ));
    assert_eq!(project.reads.load(Ordering::SeqCst), 0);
    phase.finish().unwrap();

    let elements = ryo_language::domain::elements::types::Elements::default();
    // No relation is held: the rule's read of Function is refused.
    let facts = TableFacts::new(&[]);
    let mut rules = run_file(
        &request,
        FileRun::of(
            &[FileRuleIntent::Run {
                diagnostic: DiagnosticDelivery::Collected,
                observations: ObservationDemand::NotRequired,
                capture: CaptureIntent::DiagnosticAndObservations,
            }],
            &elements,
        ),
        &mut symbols,
        &facts,
    );
    assert!(matches!(root(ran(&mut rules, 0)), Err(FileStop::Facts(_))));
    assert_eq!(facts.history(), [Relation::Function]);
    request.finish();
}

#[test]
fn a_skipped_project_frontier_preserves_the_later_original_file_error_index() {
    let engine = engine();
    let mut input = authored_request(1);
    let file_column = u32::try_from(
        kinds(Relation::Function)
            .iter()
            .position(|kind| *kind == CellKind::File)
            .unwrap(),
    )
    .unwrap();
    crate::boundary_fixture::fixtures::plans(&mut input)[0].steps = vec![
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: Input::Relation(Relation::Function),
                comparisons: vec![],
                kept: vec![ColumnIndex(file_column)],
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: Input::Relation(Relation::Function),
                comparisons: vec![],
                kept: vec![ColumnIndex(99)],
            },
        },
        PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Select {
                from: Input::Relation(Relation::ModuleIdentity),
                comparisons: vec![],
                kept: vec![ColumnIndex(0)],
            },
        },
    ];
    crate::boundary_fixture::fixtures::plans(&mut input)[0].frontier = vec![
        Input::Step(StepId(0)),
        Input::Relation(Relation::ModuleIdentity),
        Input::Step(StepId(1)),
    ];
    crate::boundary_fixture::fixtures::plans(&mut input)[0]
        .emit
        .step = StepId(2);
    let prepared = engine
        .prepare(input)
        .expect("retain each frontier's independent refusal");
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None.into());
    let table = source();
    // File capture cannot read the intervening project frontier: the facts hold Function alone.
    let facts = TableFacts::new(&[(Relation::Function, &table)]);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let mut symbols = Symbols::default();
    let mut rules = run_file(
        &request,
        FileRun::of(
            &[FileRuleIntent::Run {
                diagnostic: DiagnosticDelivery::Collected,
                observations: ObservationDemand::NotRequired,
                capture: CaptureIntent::DiagnosticAndObservations,
            }],
            &elements,
        ),
        &mut symbols,
        &facts,
    );
    let RuleAfter::Ran { captures } = ran(&mut rules, 0).after else {
        panic!("the rule runs and captures its frontiers")
    };
    assert_eq!(
        captures
            .iter()
            .map(|capture| capture.input)
            .collect::<Vec<_>>(),
        [InputNumber(0), InputNumber(2)]
    );
    assert!(
        captures[0].outcome.is_ok(),
        "independent earlier file frontier remains available"
    );
    assert!(matches!(
        &captures[1].outcome,
        Err(OperatingError::Table(RowsError::NoSuchColumn { index: 99 }))
    ));
    request.finish();
}
