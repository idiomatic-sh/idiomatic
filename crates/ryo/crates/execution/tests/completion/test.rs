//! Blocked regions, empty publications and merged coverage, through the public file session.
use crate::boundary_fixture::{authored_request, engine, source};
use crate::file_intents::{answered, partial};
use ryo_execution::inbound::use_cases::execution::interface::{
    PreparedExecutionProgram, RootOutcome,
};
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase as _;
use ryo_relations::domain::relations::fixtures::kinds;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, StepId, StepScope};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::rows::types::{RowCount, Table};

/// Function's rows without those whose node is a Comment's, by node.
fn functions_without_comments() -> PreparedExecutionProgram {
    let mut input = authored_request(1);
    let select = |relation| PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Select {
            from: Input::Relation(relation),
            comparisons: vec![],
            kept: vec![ColumnIndex(0)],
        },
    };
    crate::boundary_fixture::plans(&mut input)[0].steps = vec![
        select(Relation::Function),
        select(Relation::Comment),
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Antijoin {
                left: Input::Step(StepId(0)),
                right: Input::Step(StepId(1)),
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
            },
        },
    ];
    crate::boundary_fixture::plans(&mut input)[0].emit.step = StepId(2);
    engine().prepare(input).expect("prepare an antijoin")
}

fn empty(relation: Relation) -> Table {
    Table {
        kinds: kinds(relation).to_vec(),
        count: RowCount(0),
        cells: vec![],
    }
}

/// S4: a region whose rule's evaluation reads a relation in part in the file is blocked before it runs: it publishes the empty
/// table, partial in that file, and reads nothing.
#[test]
fn a_region_reading_a_partial_relation_is_blocked_before_it_reads() {
    let program = functions_without_comments();
    let rows = [
        (Relation::Function, source()),
        (Relation::Comment, empty(Relation::Comment)),
    ];
    let (outcome, reads) = answered(&program, &rows, &[], &[]);
    let Ok(RootOutcome::Complete(whole)) = outcome else {
        panic!("with nothing read in part the antijoin completes")
    };
    assert_eq!(whole.count, RowCount(1), "the function no comment covers");
    let partial = [partial(Relation::Comment)];
    let (outcome, reads_blocked) = answered(&program, &rows, &partial, &partial);
    let Ok(RootOutcome::Partial { table, affected }) = outcome else {
        panic!("a blocked region publishes a partial answer")
    };
    assert_eq!(
        table.count,
        RowCount(0),
        "a blocked region publishes the empty table"
    );
    assert_eq!(
        affected,
        [Some(ryo_relations::domain::relations::types::FileId(0))]
    );
    assert_eq!(reads_blocked, 0, "a blocked region reads no relation");
    assert!(reads > 0);
}

/// Function's rows, selected by a step of their own, without those whose node is a Comment's, the Comment read directly: the
/// antijoin's left input is a step, so an empty left settles it at its empty-result gate.
fn functions_without_comment_relation() -> PreparedExecutionProgram {
    let mut input = authored_request(1);
    crate::boundary_fixture::plans(&mut input)[0].steps = vec![
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: Input::Relation(Relation::Function),
                comparisons: vec![],
                kept: vec![ColumnIndex(0)],
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Antijoin {
                left: Input::Step(StepId(0)),
                right: Input::Relation(Relation::Comment),
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
            },
        },
    ];
    crate::boundary_fixture::plans(&mut input)[0].emit.step = StepId(1);
    engine()
        .prepare(input)
        .expect("prepare an antijoin of a step and a relation")
}

/// S3: an empty left input settles an antijoin at its gate, before its right input is read; the empty table it publishes keeps
/// the coverage of the region's reads: partial in the file whose right relation was read in part.
#[test]
fn an_empty_publication_keeps_the_partial_coverage_of_its_reads() {
    let program = functions_without_comment_relation();
    let rows = [
        (Relation::Function, empty(Relation::Function)),
        (Relation::Comment, empty(Relation::Comment)),
    ];
    let (outcome, reads) = answered(&program, &rows, &[], &[]);
    assert!(matches!(outcome, Ok(RootOutcome::Complete(ref table)) if table.count == RowCount(0)));
    assert_eq!(
        reads, 2,
        "the empty left input settles the antijoin before Comment is read"
    );
    let (outcome, reads) = answered(&program, &rows, &[], &[partial(Relation::Comment)]);
    let Ok(RootOutcome::Partial { table, affected }) = outcome else {
        panic!("an empty answer whose right relation was read in part is partial")
    };
    assert_eq!(table.count, RowCount(0));
    assert_eq!(
        affected,
        [Some(ryo_relations::domain::relations::types::FileId(0))]
    );
    assert_eq!(
        reads, 4,
        "the gate settles the antijoin on its empty left input, reading Comment no further"
    );
}

/// Two source locations of file 0 on the same line, at bytes 1 and 5.
fn locations() -> Table {
    use ryo_relations::domain::relations::fixtures::node_id;
    use ryo_relations::domain::relations::types::{
        CellKind, FileId, Flag, Integer, Offset, Position, Span, Value,
    };
    let kinds = kinds(Relation::SourceLocation).to_vec();
    let mut cells = vec![];
    for (row, byte) in [(1, 5), (2, 1)] {
        for (column, kind) in kinds.iter().enumerate() {
            cells.push(match kind {
                CellKind::Node => Value::Node(node_id(FileId(0), Position(row))),
                CellKind::File => Value::File(FileId(0)),
                CellKind::Span => Value::Span(Span {
                    start: Offset(0),
                    end: Offset(0),
                }),
                CellKind::Number => Value::Number(Integer(match column {
                    5 => byte,
                    _ => 1,
                })),
                CellKind::Symbol => Value::Symbol(ryo_relations::domain::terms::types::Symbol(0)),
                CellKind::Flag => Value::Flag(Flag::Lowered),
            });
        }
    }
    Table {
        kinds,
        count: RowCount(2),
        cells,
    }
}

/// The first location of each line, by byte: an anti-join of the locations over the self-join of each line's locations. With
/// `kept` empty the engine recognises it as an ordered existence; keeping `[0, 1]`, the same meaning, declines the shape.
fn first_of_each_line(kept: Vec<ColumnIndex>) -> PreparedExecutionProgram {
    engine()
        .prepare(first_of_each_line_plan(kept))
        .expect("prepare the first location of each line")
}

fn first_of_each_line_plan(
    kept: Vec<ColumnIndex>,
) -> ryo_execution::inbound::use_cases::prepare::types::PrepareRequest {
    use ryo_values::domain::plan_parts::types::{
        ComparisonOrigin, JoinColumn, Operand, PlanComparison, RepeatedRows, Side, Test,
    };
    let line = ColumnIndex(3);
    let byte = ColumnIndex(5);
    let mut input = authored_request(1);
    let steps = [
        Operation::Select {
            from: Input::Relation(Relation::SourceLocation),
            comparisons: vec![],
            kept: vec![],
        },
        Operation::Join {
            left: Input::Step(StepId(0)),
            right: Input::Step(StepId(0)),
            left_on: vec![line],
            right_on: vec![line],
            output: vec![
                JoinColumn {
                    side: Side::Left,
                    column: byte,
                },
                JoinColumn {
                    side: Side::Right,
                    column: byte,
                },
            ],
            filter: vec![],
            view: vec![],
            reach: None,
        },
        Operation::Select {
            from: Input::Step(StepId(1)),
            kept,
            comparisons: vec![PlanComparison {
                test: Test::LessThan,
                left: Operand::Column(ColumnIndex(0)),
                right: Operand::Column(ColumnIndex(1)),
                origin: ComparisonOrigin::Unrecorded,
            }],
        },
        Operation::Project {
            from: Input::Step(StepId(2)),
            columns: vec![ColumnIndex(0)],
            distinct: RepeatedRows::Dropped,
        },
        Operation::Antijoin {
            left: Input::Step(StepId(0)),
            right: Input::Step(StepId(3)),
            left_on: vec![byte],
            right_on: vec![ColumnIndex(0)],
        },
    ];
    let plan = &mut crate::boundary_fixture::plans(&mut input)[0];
    plan.steps = steps
        .into_iter()
        .map(|operation| PlanStep {
            scope: StepScope::PerFile,
            operation,
        })
        .collect();
    plan.emit.step = StepId(4);
    input
}

/// S6: an ordered existence answers what the same plan answers with the shape declined, the whole read and read in part: under
/// a partial read of SourceLocation in file 0 both are partial in the file and in the repository.
#[test]
fn a_recognised_ordered_existence_answers_as_its_declined_twin() {
    let recognised = first_of_each_line(vec![]);
    let declined = first_of_each_line(vec![ColumnIndex(0), ColumnIndex(1)]);
    let rows = [(Relation::SourceLocation, locations())];
    let shown = |outcome: Result<RootOutcome, _>| match outcome {
        Ok(RootOutcome::Complete(table)) => format!("complete {table:?}"),
        Ok(RootOutcome::Partial { table, affected }) => format!("partial {table:?} {affected:?}"),
        Ok(RootOutcome::Failed(error)) => format!("failed {error:?}"),
        Ok(RootOutcome::Unknown(error)) => format!("unknown {error:?}"),
        Ok(_) => "other".to_owned(),
        Err(error) => format!("refused {error:?}"),
    };
    for publication in [vec![], vec![partial(Relation::SourceLocation)]] {
        assert_eq!(
            shown(answered(&recognised, &rows, &[], &publication).0),
            shown(answered(&declined, &rows, &[], &publication).0),
            "the recognised plan answers as its declined twin; read in part: {}",
            !publication.is_empty()
        );
    }
    let (outcome, _) = answered(
        &recognised,
        &rows,
        &[],
        &[partial(Relation::SourceLocation)],
    );
    let Ok(RootOutcome::Partial { affected, .. }) = outcome else {
        panic!("an ordered existence read in part answers in part")
    };
    assert_eq!(
        affected,
        [
            None,
            Some(ryo_relations::domain::relations::types::FileId(0))
        ]
    );
}
