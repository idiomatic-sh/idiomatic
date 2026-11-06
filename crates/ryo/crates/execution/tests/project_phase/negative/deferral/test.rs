//! Deferred roots do not hide owning-phase faults or renumber file frontier refusals.
use super::super::super::source_facts::fixtures::ProjectFacts;
use super::super::fixtures::NoFileFrontiers;
use crate::boundary_fixture::{
    authored_request, corpus, demand, engine, prepared, project_root, source,
};
use ryo_execution::inbound::use_cases::execution::{
    interface::{ExecutionRequest, ExecutionUseCase, FileExecution, ProjectStage},
    types::*,
};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::{
    service::kinds,
    types::{CellKind, FileId, Relation, Words, WorkerNumber},
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
    let request = engine.begin(&prepared, &corpus);
    let covered = [Covered {
        file: FileId(0),
        rules: vec![RuleIndex(0)],
        partial: vec![],
    }];
    let phase = request.project_phase().unwrap();
    let scope = phase
        .project_scope(
            RuleIndex(0),
            &corpus,
            &covered,
            &[],
            &ryo_execution::inbound::use_cases::execution::types::InapplicableFiles::default(),
        )
        .unwrap();
    let project = ProjectFacts::new(Relation::ModuleIdentity);
    let mut symbols = Symbols::default();
    let project_symbols = Symbols::default();
    let ordinary = request
        .prepare_frontiers(
            FrontierRequest {
                demand: FrontierDemand::Diagnostic,
                scope: &scope,
                rule: RuleIndex(0),
                first: Bucket(0),
                facts: &NoFileFrontiers,
                project: &project,
                project_symbols: &project_symbols,
            },
            &mut symbols,
        )
        .unwrap();
    let partials = Partials::default();
    let result = project_root(
        &phase,
        RuleIndex(0),
        demand(),
        ProjectInput {
            scope: &scope,
            evaluation: &partials,
            publication: &partials,
            ordinary,
            bound: &[],
            arenas: &[],
        },
        &mut symbols,
    )
    .unwrap();
    assert!(matches!(
        result.emit,
        RootOutcome::Deferred {
            phase: StepScope::PerFile
        }
    ));
    assert!(result.counts.fed.iter().all(|count| count.0 == [0; 3]));
    assert_eq!(project.reads.load(Ordering::SeqCst), 0);
    drop(scope);
    phase.finish().unwrap();

    let elements = ryo_language::domain::elements::types::Elements::default();
    let read = |actual| {
        assert_eq!(actual, Relation::Function);
        Err(OperatingError::NotRead)
    };
    let mut file = request
        .open_file(
            FileInput {
                language: SourceLanguage::Recognized,
                resident_stability: facts::ResidentStability::Stable,
                file: FileId(0),
                evaluation: &[],
                publication: &[],
                intents: &[FileRuleIntent::Run {
                    diagnostic: DiagnosticDelivery::Collected,
                    observations: ObservationDemand::NotRequired,
                    capture: CaptureIntent::Disabled,
                }],
                source: b"abc",
                elements: &elements,
                read: &read,
                semantic: None,
                kinds: None,
            },
            &mut symbols,
        )
        .unwrap();
    assert!(matches!(
        file.rule(RuleIndex(0), demand(), &mut symbols)
            .unwrap()
            .emit,
        RootOutcome::Failed(OperatingError::NotRead)
    ));
    file.finish().unwrap();
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
    input.plans[0].steps = vec![
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
    input.plans[0].frontier = vec![
        Input::Step(StepId(0)),
        Input::Relation(Relation::ModuleIdentity),
        Input::Step(StepId(1)),
    ];
    input.plans[0].emit.step = StepId(2);
    let prepared = engine
        .prepare(input)
        .expect("retain each frontier's independent refusal");
    let corpus = corpus();
    let request = engine.begin(&prepared, &corpus);
    let table = source();
    let read = |actual| {
        assert_eq!(
            actual,
            Relation::Function,
            "file capture cannot read the intervening project frontier"
        );
        Ok(SourceRows::Computed(&table))
    };
    let elements = ryo_language::domain::elements::types::Elements::default();
    let mut symbols = Symbols::default();
    let mut file = request
        .open_file(
            FileInput {
                language: SourceLanguage::Recognized,
                resident_stability: facts::ResidentStability::Stable,
                file: FileId(0),
                evaluation: &[],
                publication: &[],
                intents: &[FileRuleIntent::Run {
                    diagnostic: DiagnosticDelivery::Collected,
                    observations: ObservationDemand::NotRequired,
                    capture: CaptureIntent::DiagnosticAndObservations,
                }],
                source: b"abc",
                elements: &elements,
                read: &read,
                semantic: None,
                kinds: None,
            },
            &mut symbols,
        )
        .unwrap();
    let mut arena = vec![];
    let mut places = vec![];
    let mut words = Words::default();
    let captures = file
        .capture_frontiers(
            RuleIndex(0),
            Bucket(0),
            &mut symbols,
            CaptureContext {
                worker: WorkerNumber(0),
                arena: &mut arena,
                places: &mut places,
                words: &mut words,
            },
        )
        .unwrap();
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
    file.finish().unwrap();
    request.finish();
}
