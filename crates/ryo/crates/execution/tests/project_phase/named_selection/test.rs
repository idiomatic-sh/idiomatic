//! Named function selection replays without running project steps or hiding absence.
use crate::boundary_fixture::fixtures::{authored_request, corpus, demand, engine};
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::ProjectStage;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution_runtime::application::file_dag::program::file_session::interface::FileExecution;
use ryo_execution_runtime::application::file_dag::program::file_session::types::CaptureContext;
use ryo_execution_runtime::application::file_dag::program::file_session::types::FileInput;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceLanguage;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceRows;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{FileId, Relation, Value};
use ryo_relations::domain::relations::types::{Words, WorkerNumber};
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input, StepId, StepScope},
    plans::types::{Operation, PlanStep},
    rows::types::RowCount,
};
use std::cell::Cell;

/// The former per-file, missing-input and shared-pool scenarios use one prepared program.
#[test]
fn named_function_selection_replays_without_running_project_steps_or_hiding_absence() {
    use ryo_relations::domain::relations::{fixtures::node_id, types::Position};
    use ryo_values::domain::plan_parts::types::{
        LiteralText, Operand, PlanComparison, QueryLiteral, Test,
    };
    for absent in [false, true] {
        let engine = engine();
        let mut input = authored_request(2);
        let selected = PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: Input::Relation(Relation::Function),
                comparisons: vec![PlanComparison {
                    test: Test::Equal,
                    left: Operand::Column(ColumnIndex(1)),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText("main".into()))),
                    origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                }],
                kept: vec![
                    ColumnIndex(0),
                    ColumnIndex(1),
                    ColumnIndex(2),
                    ColumnIndex(3),
                ],
            },
        };
        let projected = PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Project {
                from: Input::Step(StepId(0)),
                columns: vec![ColumnIndex(0)],
                distinct: ryo_values::domain::plan_parts::types::RepeatedRows::Dropped,
            },
        };
        crate::boundary_fixture::fixtures::plans(&mut input)[0].steps =
            vec![selected.clone(), projected.clone()];
        crate::boundary_fixture::fixtures::plans(&mut input)[0]
            .emit
            .step = StepId(1);
        crate::boundary_fixture::fixtures::plans(&mut input)[1].steps = vec![
            selected,
            projected,
            PlanStep {
                scope: StepScope::CrossFile,
                operation: Operation::GroupCount {
                    from: Input::Step(StepId(1)),
                    keys: vec![ColumnIndex(0)],
                    distinct: None,
                },
            },
        ];
        crate::boundary_fixture::fixtures::plans(&mut input)[1].frontier =
            vec![Input::Step(StepId(1))];
        crate::boundary_fixture::fixtures::plans(&mut input)[1]
            .emit
            .step = StepId(2);
        let program = engine.prepare(input).expect("compile both plans together");
        let mut corpus = corpus();
        if let ryo_values::domain::files::types::CorpusInput::Available(entry) = &mut corpus[0] {
            entry.size = ryo_values::domain::files::types::FileSize(9);
        }
        let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
        let mut symbols = Symbols::default();
        let table = crate::boundary_fixture::fixtures::named_functions(&mut symbols);
        let reads = Cell::new(0);
        let read = |relation| {
            assert_eq!(relation, Relation::Function);
            reads.set(reads.get() + 1);
            if absent {
                Err(OperatingError::NotRead)
            } else {
                Ok(SourceRows::Computed(&table))
            }
        };
        let elements = ryo_language::domain::elements::types::Elements::default();
        let mut file = request
            .open_file(
                FileInput {
                    language: SourceLanguage::Recognized,
                    resident_stability: ResidentStability::Stable,
                    file: FileId(0),
                    evaluation: &[],
                    publication: &[],
                    intents: &[
                        FileRuleIntent::Run {
                            diagnostic: DiagnosticDelivery::Collected,
                            observations: ObservationDemand::NotRequired,
                            capture: CaptureIntent::DiagnosticAndObservations,
                        },
                        FileRuleIntent::Run {
                            diagnostic: DiagnosticDelivery::Collected,
                            observations: ObservationDemand::NotRequired,
                            capture: CaptureIntent::DiagnosticAndObservations,
                        },
                    ],
                    source: ryo_values::domain::files::types::SourceView(b"mainother"),
                    elements: &elements,
                    read: &read,
                    semantic: ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport::Absent,
                    kinds: ryo_execution_runtime::application::file_dag::program::file_session::types::KindSupport::Absent,
                },
                &mut symbols,
            )
            .unwrap();
        let outcome = file.rule(RuleIndex(0), demand(), &mut symbols).unwrap();
        if absent {
            assert!(matches!(
                outcome.emit,
                RootOutcome::Failed(OperatingError::NotRead)
            ));
        } else {
            let RootOutcome::Complete(first) = outcome.emit else {
                panic!("matching declarations")
            };
            assert_eq!(first.count, RowCount(2));
            assert_eq!(
                first.cells,
                [
                    Value::Node(node_id(FileId(0), Position(0))),
                    Value::Node(node_id(FileId(0), Position(1)))
                ]
            );
            let after_first = reads.get();
            assert!(matches!(
                file.rule(RuleIndex(1), demand(), &mut symbols)
                    .unwrap()
                    .emit,
                RootOutcome::Deferred {
                    phase: StepScope::CrossFile
                }
            ));
            let mut arena = vec![];
            let mut places = vec![];
            let mut words = Words::default();
            let mut captured = file
                .capture_frontiers(
                    RuleIndex(1),
                    Bucket(0),
                    &mut symbols,
                    CaptureContext {
                        worker: WorkerNumber(0),
                        arena: ryo_execution::inbound::use_cases::execution::interface::Arena(
                            &mut arena,
                        ),
                        places: &mut places,
                        words: &mut words,
                    },
                )
                .expect("project rule retains its file frontier");
            assert_eq!(captured.len(), 1);
            let capture = captured.pop().unwrap().outcome.expect("complete frontier");
            let decoded = ryo_values::domain::table_words::fixtures::decode_complete(
                &capture.words,
                &mut vec![],
            )
            .unwrap();
            assert_eq!(
                decoded, first,
                "captured rows match the actual file selection"
            );
            let RootOutcome::Complete(replayed) = file
                .rule(RuleIndex(0), demand(), &mut symbols)
                .unwrap()
                .emit
            else {
                panic!("repeat file rule")
            };
            assert_eq!(first, replayed);
            assert_eq!(
                reads.get(),
                after_first,
                "deferred and replayed roots do not reread source"
            );
        }
        file.finish().expect("finish actual file session");
        request.finish();
    }
}
