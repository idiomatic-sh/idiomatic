//! A compiled project emit defers at file scope while real file frontiers remain available.
use crate::boundary_fixture::{authored_request, corpus, demand, engine, source};
use ryo_execution::inbound::use_cases::execution::interface::{ProjectExecution, ProjectStage};
use ryo_execution::inbound::use_cases::execution::{
    interface::{ExecutionRequest, ExecutionUseCase, FileExecution},
    types::*,
    use_case::ExecutionUseCaseImpl,
};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{Words, WorkerNumber};
use ryo_relations::domain::relations::{
    service::kinds,
    types::{CellKind, FileId, Relation, Value},
};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input, StepId},
    plans::types::{Operation, PlanStep, StepScope},
    rows::types::RowCount,
};
use ryo_values::domain::{
    plan_parts::types::{InputNumber, JoinColumn, Side},
    stored_facts::types::Bucket,
};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};
mod source_facts;
use self::source_facts::{CapturedFacts, ProjectFacts};

fn mixed_program(engine: &ExecutionUseCaseImpl, relation: Relation) -> PreparedProgram {
    let mut input = authored_request(1);
    let file_column = kinds(Relation::Function)
        .iter()
        .position(|kind| *kind == CellKind::File)
        .unwrap();
    input.plans[0].steps = vec![
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: Input::Relation(Relation::Function),
                comparisons: vec![],
                kept: vec![ColumnIndex(u32::try_from(file_column).unwrap())],
            },
        },
        PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Select {
                from: Input::Relation(relation),
                comparisons: vec![],
                kept: vec![ColumnIndex(0)],
            },
        },
        PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Join {
                left: Input::Step(StepId(0)),
                right: Input::Step(StepId(1)),
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
                output: vec![JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(0),
                }],
                filter: vec![],
                view: vec![],
                reach: None,
            },
        },
    ];
    input.plans[0].frontier = vec![Input::Step(StepId(0)), Input::Relation(relation)];
    input.plans[0].emit.step = StepId(2);
    engine
        .prepare(input)
        .expect("compile actual mixed-phase request")
}

#[test]
fn project_relations_defer_at_file_scope_and_join_real_captured_rows_once() {
    for relation in [
        Relation::ProjectNamespaceMembership,
        Relation::ModuleIdentity,
        Relation::ManifestDiagnostic,
    ] {
        let engine = engine();
        let prepared = mixed_program(&engine, relation);
        let corpus = corpus();
        let request = engine.begin(&prepared, &corpus);
        let table = source();
        let file_reads = Cell::new(0);
        let read = |actual| {
            assert_eq!(
                actual,
                Relation::Function,
                "project facts must never be read by the file owner"
            );
            file_reads.set(file_reads.get() + 1);
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
                    demanded: &[true],
                    source: b"abc",
                    elements: &elements,
                    read: &read,
                    semantic: None,
                    kinds: None,
                },
                &mut symbols,
            )
            .expect("open actual file owner");
        assert!(matches!(
            file.rule(RuleIndex(0), demand(), &mut symbols)
                .unwrap()
                .emit,
            RootOutcome::Deferred {
                phase: StepScope::CrossFile
            }
        ));
        let mut emitted = vec![];
        assert!(matches!(
            file.emit_rows(RuleIndex(0), &mut symbols, &mut |row| {
                emitted.extend_from_slice(row);
                Ok(())
            })
            .unwrap(),
            RowEmission::Deferred {
                phase: StepScope::CrossFile
            }
        ));
        assert!(emitted.is_empty());
        assert_eq!(file_reads.get(), 0);
        assert!(
            file.counts(RuleIndex(0))
                .unwrap()
                .fed
                .iter()
                .all(|count| count.0 == [0; 3])
        );
        let mut arena = vec![];
        let mut places = vec![];
        let mut words = Words::default();
        let mut captures = file
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
            .expect("file frontier still executes");
        assert_eq!(
            captures.len(),
            1,
            "project-owned frontier is not captured as a file input"
        );
        assert_eq!(captures[0].input, InputNumber(0));
        let capture = captures
            .pop()
            .unwrap()
            .outcome
            .expect("actual file frontier");
        // Admission, the declined stored-selection candidate and registered
        // evaluation borrow the same pinned computed population. These are
        // resident resolutions, not three provider population builds.
        assert_eq!(file_reads.get(), 3);
        assert_eq!(file.counts(RuleIndex(0)).unwrap().fed[1].0, [0; 3]);
        file.finish()
            .expect("file owner finishes without demanding project sources");
        assert_eq!(file_reads.get(), 3, "finish cannot demand the source again");
        let stored = CapturedFacts {
            capture,
            reads: AtomicUsize::new(0),
        };
        let project = ProjectFacts::new(relation);
        let covered = [Covered {
            file: FileId(0),
            rules: vec![RuleIndex(0)],
            partial: vec![],
        }];
        let scope = request
            .project_scope(RuleIndex(0), &corpus, &covered, &[])
            .unwrap();
        let project_symbols = Symbols::default();
        let mut symbols = Symbols::default();
        let ordinary = request
            .prepare_frontiers(
                FrontierRequest {
                    demand: FrontierDemand::Diagnostic,
                    scope: &scope,
                    rule: RuleIndex(0),
                    first: Bucket(0),
                    facts: &stored,
                    project: &project,
                    project_symbols: &project_symbols,
                },
                &mut symbols,
            )
            .expect("admit real file frontier and project facts");
        assert_eq!(stored.reads.load(Ordering::SeqCst), 1);
        assert_eq!(project.reads.load(Ordering::SeqCst), 1);
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
        let outcome = run
            .rule(RuleIndex(0), demand(), &mut symbols)
            .expect("project emit runs at project scope");
        let RootOutcome::Complete(table) = outcome.emit else {
            panic!("one complete project diagnostic table")
        };
        assert_eq!(table.kinds, [CellKind::File]);
        assert_eq!(table.count, RowCount(1));
        assert_eq!(table.cells, [Value::File(FileId(0))]);
        let counts = outcome
            .counts
            .fed
            .iter()
            .map(|count| count.0)
            .collect::<Vec<_>>();
        let again = run.rule(RuleIndex(0), demand(), &mut symbols).unwrap();
        assert_eq!(
            again
                .counts
                .fed
                .iter()
                .map(|count| count.0)
                .collect::<Vec<_>>(),
            counts,
            "one cached project invocation, with no second semantic evaluation"
        );
        run.finish();
        scope.finish().expect("release project publications");
        assert_eq!(project.reads.load(Ordering::SeqCst), 1);
        request.finish();
        assert_eq!(file_reads.get(), 3, "project work cannot reread file facts");
    }
}
