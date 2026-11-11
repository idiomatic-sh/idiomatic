//! Closed empty declarations remain typed populations; absent declarations remain refusals.
use crate::boundary_fixture::fixtures::{authored_request, corpus, demand, engine};
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::Covered;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileExecution;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::Partials;
use ryo_execution::inbound::use_cases::execution::interface::ProjectStage;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::interface::RuleIntentError;
use ryo_execution::inbound::use_cases::execution::interface::{
    BatchNumber, BatchOccurrence, ObservationBatches,
};
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::ObservationIdentity;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution::inbound::use_cases::prepare::types::PrepareRequest;
use ryo_execution_runtime::application::file_dag::program::file_session::types::CaptureContext;
use ryo_execution_runtime::application::file_dag::program::file_session::types::FileInput;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceLanguage;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceRows;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_runtime::outbound::facts::fixtures::project_phase_source_facts::CapturedFacts;
use ryo_execution_runtime::outbound::facts::fixtures::project_phase_source_facts::ProjectFacts;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, FileId, Relation};
use ryo_relations::domain::relations::types::{Words, WorkerNumber};
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::{
    observations::types::{
        BoundObservationInput, ObservationBinding, ObservationBindingName, ObservationColumn,
        ObservationColumnName, ObservationName, ObservationProducer,
    },
    plan_parts::types::ObservationBindingId,
    runs::types::RuleId,
    stored_facts::types::Digest,
};
use ryo_values::domain::{
    plan_parts::types::{Input, StepScope},
    plans::types::{Operation, PlanStep},
    rows::types::RowCount,
};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

fn declared() -> PrepareRequest {
    let mut input = authored_request(1);
    let columns = vec![ObservationColumn {
        name: ObservationColumnName("code".into()),
        kind: CellKind::Symbol,
    }];
    input.rules[0].observations.push(ObservationBinding {
        name: ObservationBindingName("codes".into()),
        columns: columns.clone(),
        producers: vec![],
    });
    crate::boundary_fixture::fixtures::plans(&mut input)[0]
        .observation_inputs
        .push(BoundObservationInput {
            name: ObservationBindingName("codes".into()),
            columns,
            scope: StepScope::CrossFile,
        });
    crate::boundary_fixture::fixtures::plans(&mut input)[0].steps = vec![PlanStep {
        scope: StepScope::CrossFile,
        operation: Operation::Select {
            from: Input::BoundObservation(ObservationBindingId(0)),
            comparisons: vec![],
            kept: vec![],
        },
    }];
    crate::boundary_fixture::fixtures::plans(&mut input)[0].frontier =
        vec![Input::BoundObservation(ObservationBindingId(0))];
    input.observations = ObservationIdentity::Named(
        ryo_values::domain::records::types::ObservationReadinessLayout(Digest([9; 16])),
    );
    input
}

#[test]
fn declared_empty_observation_defers_in_file_and_publishes_typed_empty_in_project() {
    let engine = engine();
    let prepared = engine
        .prepare(declared())
        .expect("closed empty declaration");
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None);
    let reads = Cell::new(0);
    let read = |_| {
        reads.set(reads.get() + 1);
        Err::<SourceRows<'_>, _>(OperatingError::NotRead)
    };
    let elements = ryo_language::domain::elements::types::Elements::default();
    let mut symbols = Symbols::default();
    let mut file = request
        .open_file(
            FileInput {
                language: SourceLanguage::Recognized,
                resident_stability: ResidentStability::Stable,
                file: FileId(0),
                evaluation: &[],
                publication: &[],
                intents: &[FileRuleIntent::Run {
                    diagnostic: DiagnosticDelivery::Collected,
                    observations: ObservationDemand::NotRequired,
                    capture: CaptureIntent::DiagnosticAndObservations,
                }],
                source: ryo_values::domain::files::types::SourceView(b"abc"),
                elements: &elements,
                read: &read,
                semantic: ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport::Absent,
                kinds: ryo_execution_runtime::application::file_dag::program::file_session::types::KindSupport::Absent,
            },
            &mut symbols,
        )
        .unwrap();
    assert!(matches!(
        file.rule(RuleIndex(0), demand(), &mut symbols)
            .unwrap()
            .emit,
        RootOutcome::Deferred {
            phase: StepScope::CrossFile
        }
    ));
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
                arena: ryo_execution::inbound::use_cases::execution::interface::Arena(&mut arena),
                places: &mut places,
                words: &mut words,
            },
        )
        .unwrap();
    assert!(
        captures.is_empty(),
        "project population is not fabricated as a file frame"
    );
    assert_eq!(reads.get(), 0);
    file.finish().unwrap();
    assert_eq!(reads.get(), 0);

    let stored = CapturedFacts {
        captures: vec![],
        reads: AtomicUsize::new(0),
    };
    let project = ProjectFacts::new(Relation::ModuleIdentity);
    let covered = [Covered {
        file: FileId(0),
        rules: vec![RuleIndex(0)],
        partial: vec![],
    }];
    let mut arenas = vec![];
    let stage = request.observations(&mut arenas).unwrap();
    let project_symbols = Symbols::default();
    let partials = Partials::default();
    let inapplicable =
        [ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles::default()];
    let refusals =
        [ryo_execution::inbound::use_cases::execution::interface::RefusedFiles::default()];
    let (ran, delivered) = crate::project_phase::source_facts::fixtures::run_observations(
        stage,
        ObservationBatches {
            occurrences: &[BatchOccurrence {
                batch: BatchNumber(0),
                rule: RuleIndex(0),
            }],
            readiness: &[],
            rules: &[RuleIndex(0)],
            first: &[Bucket(0)],
            corpus: &corpus,
            covered: &covered,
            skipped: &[],
            inapplicable: &inapplicable,
            refused: &refusals,
            facts: &stored,
            project: &project,
            project_symbols: &project_symbols,
        },
        &partials,
        &mut symbols,
    );
    ran.unwrap();
    assert_eq!(delivered.len(), 1);
    for (_, outcome) in delivered {
        let RootOutcome::Complete(table) = outcome.unwrap().emit else {
            panic!("authoritatively empty union must complete in its declared phase")
        };
        assert_eq!(table.kinds, [CellKind::Symbol]);
        assert_eq!(table.count, RowCount(0));
        assert!(table.cells.is_empty());
    }
    assert_eq!(stored.reads.load(Ordering::SeqCst), 0);
    assert_eq!(project.reads.load(Ordering::SeqCst), 0);
}

#[test]
fn no_query_is_unavailable_rather_than_an_empty_population() {
    let mut input = declared();
    input.rules[0].observations[0].producers = vec![ObservationProducer::NoQuery {
        rule: RuleId("not-executable".into()),
    }];
    // The producer has no query, so it never publishes: the consumer's frontier is unavailable before its schema is compared.
    project_refuses(
        input,
        "a producer feeding this retained frontier is unavailable",
    );
}

fn project_refuses(input: PrepareRequest, expected: &str) {
    let engine = engine();
    let prepared = engine
        .prepare(input)
        .expect("valid declaration with deferred admission refusal");
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None);
    let stored = CapturedFacts {
        captures: vec![],
        reads: AtomicUsize::new(0),
    };
    let project = ProjectFacts::new(Relation::ModuleIdentity);
    let covered = [Covered {
        file: FileId(0),
        rules: vec![RuleIndex(0)],
        partial: vec![],
    }];
    let mut arenas = vec![];
    let stage = request.observations(&mut arenas).unwrap();
    let mut symbols = Symbols::default();
    let project_symbols = Symbols::default();
    let partials = Partials::default();
    let (ran, mut delivered) = crate::project_phase::source_facts::fixtures::run_observations(
        stage,
        ObservationBatches {
            occurrences: &[BatchOccurrence {
                batch: BatchNumber(0),
                rule: RuleIndex(0),
            }],
            readiness: &[],
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
            facts: &stored,
            project: &project,
            project_symbols: &project_symbols,
        },
        &partials,
        &mut symbols,
    );
    ran.unwrap();
    let (_, outcome) = delivered.pop().unwrap();
    assert!(matches!(outcome.unwrap().emit,
        RootOutcome::Unknown(OperatingError::ObservationIncomplete { ref reason })
        if reason == expected));
    assert_eq!(stored.reads.load(Ordering::SeqCst), 0);
    assert_eq!(project.reads.load(Ordering::SeqCst), 0);
}

#[test]
fn missing_declarations_outputs_and_mismatched_schemas_are_not_closed_empty() {
    let engine = engine();
    let mut missing = declared();
    missing.rules[0].observations.clear();
    assert!(matches!(
        engine.prepare(missing),
        Err(OperatingError::ShapeMismatch)
    ));
    let mut missing_slot = declared();
    crate::boundary_fixture::fixtures::plans(&mut missing_slot)[0].steps[0].operation =
        Operation::Select {
            from: Input::BoundObservation(ObservationBindingId(1)),
            comparisons: vec![],
            kept: vec![],
        };
    assert!(matches!(
        engine.prepare(missing_slot),
        Err(OperatingError::ObservationConfiguration(
            RuleIntentError::InvalidReference
        ))
    ));
    let mut mismatch = declared();
    crate::boundary_fixture::fixtures::plans(&mut mismatch)[0].observation_inputs[0].columns[0]
        .kind = CellKind::Number;
    project_refuses(
        mismatch,
        "observation does not match its actual consumer declaration",
    );

    let mut unknown = declared();
    unknown.rules[0].observations[0].producers = vec![ObservationProducer::Output {
        rule: RuleId("missing".into()),
        output: ObservationName("codes".into()),
    }];
    assert!(matches!(
        engine.prepare(unknown),
        Err(OperatingError::ObservationConfiguration(
            RuleIntentError::UnknownProducer { .. }
        ))
    ));
    let mut missing_output = declared();
    let other = authored_request(2);
    missing_output.rules.push(other.rules[1].clone());
    crate::boundary_fixture::fixtures::plans(&mut missing_output)
        .push(crate::boundary_fixture::fixtures::plans_of(&other)[1].clone());
    missing_output.digests.push(other.digests[1]);
    crate::boundary_fixture::fixtures::bindings(&mut missing_output).push(vec![]);
    missing_output.rules[0].observations[0].producers = vec![ObservationProducer::Output {
        rule: missing_output.rules[1].id.clone(),
        output: ObservationName("codes".into()),
    }];
    assert!(matches!(
        engine.prepare(missing_output),
        Err(OperatingError::ObservationConfiguration(
            RuleIntentError::MissingOutput { .. }
        ))
    ));
}
