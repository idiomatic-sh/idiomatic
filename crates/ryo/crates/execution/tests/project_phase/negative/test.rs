//! Deferred roots do not hide owning-phase faults or renumber file frontier refusals.
use super::super::source_facts::fixtures::ProjectFacts;
use super::fixtures::{Entry, Root, submit};
use crate::boundary_fixture::{authored_request, corpus, demand, engine, prepared, source};
use ryo_execution::inbound::use_cases::execution::{
    interface::{
        ExecutionRequest, ExecutionUseCase, FileExecution, ProjectExecution, ProjectStage,
    },
    types::*,
};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::{
    service::kinds,
    types::{CellKind, FileId, Relation, Words, WorkerNumber},
};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input, InputNumber, StepId},
    plans::types::{Operation, PlanStep, StepScope},
    rows::errors::TableModelError,
    stored_facts::types::Bucket,
};
use std::sync::atomic::Ordering;

struct NoFileFrontiers;
impl facts::FactsFinish for NoFileFrontiers {
    fn kept(&self, _: Bucket) -> Result<facts::KeptBucket, facts::FactsError> {
        panic!("this compiled rule has no file frontier to fetch")
    }
}

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
        .project_scope(RuleIndex(0), &corpus, &covered, &[])
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
    let mut run = request
        .open_project(
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
    let result = run.rule(RuleIndex(0), demand(), &mut symbols).unwrap();
    assert!(matches!(
        result.emit,
        RootOutcome::Deferred {
            phase: StepScope::PerFile
        }
    ));
    assert!(result.counts.fed.iter().all(|count| count.0 == [0; 3]));
    assert_eq!(project.reads.load(Ordering::SeqCst), 0);
    run.finish();
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
        Err(OperatingError::Table(TableModelError::NoSuchColumn {
            index: 99
        }))
    ));
    file.finish().unwrap();
    request.finish();
}

fn root(rule: usize) -> Root {
    Root {
        declared: rule,
        named: rule,
    }
}

#[test]
fn an_omitted_declared_root_gets_no_outcome_and_no_guard_and_every_submitted_root_gets_one() {
    let engine = engine();
    let all = [root(0), root(1), root(2)];
    for count in 0..=all.len() {
        let submitted = submit(&engine, Entry::Phase, 3, &all[..count], None);
        assert!(submitted.refused.is_none());
        assert_eq!(
            submitted.outcomes.len(),
            count,
            "one outcome per submitted root, none invented for the rest"
        );
        assert!(submitted.outcomes.iter().all(Result::is_ok));
    }
    let omitted = submit(&engine, Entry::Phase, 3, &[root(0), root(2)], None);
    assert_eq!(omitted.outcomes.len(), 2);
    assert!(
        !omitted.guarded.contains(&RuleIndex(1)),
        "the omitted root is never admitted, guarded or delivered"
    );
}

#[test]
fn an_extra_root_is_refused_by_name_without_displacing_the_declared_roots() {
    let engine = engine();
    let extra = Root {
        declared: 0,
        named: 3,
    };
    let submitted = submit(&engine, Entry::Phase, 3, &[root(0), extra, root(2)], None);
    assert!(submitted.refused.is_none());
    assert_eq!(submitted.outcomes.len(), 3, "the extra root keeps its own slot");
    assert!(submitted.outcomes[0].is_ok());
    assert!(
        matches!(submitted.outcomes[1], Err(OperatingError::NotRun)),
        "a rule the program never declared is a typed refusal"
    );
    assert!(submitted.outcomes[2].is_ok());

    let guarded = submit(&engine, Entry::Phase, 3, &[root(0), extra, root(2)], Some(RuleIndex(3)));
    assert!(matches!(guarded.refused, Some(OperatingError::WrongBinding)));
    assert_eq!(
        guarded.outcomes.len(),
        1,
        "refusal at the extra root completes only the roots declared before it"
    );
    assert!(!guarded.guarded.contains(&RuleIndex(2)));
}
