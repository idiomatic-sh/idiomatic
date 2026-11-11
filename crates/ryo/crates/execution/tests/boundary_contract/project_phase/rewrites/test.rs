//! Regrouping, narrowing and selection pushing each leave the rows a plan answers over several files as they were. A span states no
//! file and span equality is value equality, so a join on spans alone after the files pairs functions of two files whose spans are
//! equal. The compiler refuses such a step (`LoweringError::CrossFileSpanKey`: a step keyed on spans alone needs per-file input);
//! the plans here are built by hand to show the rewrites still leave it after the files, never making it in each file, where it would
//! answer other rows. A join on the file is made in each file. Narrowing cuts a per-file input by the nodes or files the other input
//! holds, never by a span.
use super::fixtures::{FILE, ID, NAME, SPAN, files, functions_kept, rows};
use ryo_query_language::inbound::use_cases::compile::fixtures::rewrites::{
    narrowed, regrouped, selections_pushed,
};
use ryo_relations::domain::relations::types::{FileId, Relation, Value};
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plan_parts::types::{
    ComparisonOrigin, LiteralText, Operand, PlanComparison, QueryLiteral, Test,
};
use ryo_values::domain::plan_parts::types::{JoinColumn, Side};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input, StepId, StepScope},
    plans::types::{Operation, PlanStep},
};

/// A step after the files.
fn after_the_files(operation: Operation) -> PlanStep {
    PlanStep {
        scope: StepScope::CrossFile,
        operation,
    }
}

/// A step made in each file.
fn in_each_file(operation: Operation) -> PlanStep {
    PlanStep {
        scope: StepScope::PerFile,
        operation,
    }
}

fn column(side: Side, at: ColumnIndex) -> JoinColumn {
    JoinColumn { side, column: at }
}

fn join(
    left: StepId,
    right: StepId,
    keys: (ColumnIndex, ColumnIndex),
    output: Vec<JoinColumn>,
) -> Operation {
    Operation::Join {
        left: Input::Step(left),
        right: Input::Step(right),
        left_on: vec![keys.0],
        right_on: vec![keys.1],
        output,
        filter: vec![],
        view: vec![],
        reach: None,
    }
}

/// The distinct values of one column of a per-file step, gathered after the files.
fn gathered(from: StepId, at: ColumnIndex) -> PlanStep {
    after_the_files(Operation::Project {
        from: Input::Step(from),
        columns: vec![at],
        distinct: RepeatedRows::Dropped,
    })
}

/// The functions named `main`, in each file.
fn named_main(kept: &[ColumnIndex]) -> PlanStep {
    in_each_file(Operation::Select {
        from: Input::Relation(Relation::Function),
        comparisons: vec![PlanComparison {
            test: Test::Equal,
            left: Operand::Column(NAME),
            right: Operand::Literal(QueryLiteral::Text(LiteralText("main".into()))),
            origin: ComparisonOrigin::Unrecorded,
        }],
        kept: kept.to_vec(),
    })
}

/// `Join(Join(spans, functions on the span), functions on key)`: the spans of every function gathered after the files, met with
/// the functions that have them, and those met with the functions again on the outer key, a column of each side by place in the
/// selection (id, file, span). The answer is the two functions of each pair.
fn chained(key: ColumnIndex) -> (Vec<PlanStep>, StepId) {
    let kept = [ID, FILE, SPAN];
    let at = |place: u32| ColumnIndex(place);
    (
        vec![
            functions_kept(&[SPAN]),
            gathered(StepId(0), at(0)),
            functions_kept(&kept),
            after_the_files(join(
                StepId(1),
                StepId(2),
                (at(0), at(2)),
                vec![
                    column(Side::Right, at(0)),
                    column(Side::Right, at(1)),
                    column(Side::Right, at(2)),
                ],
            )),
            functions_kept(&kept),
            after_the_files(join(
                StepId(3),
                StepId(4),
                (key, key),
                vec![column(Side::Left, at(0)), column(Side::Right, at(0))],
            )),
        ],
        StepId(5),
    )
}

/// Whether some row holds the function at `place` of `file`.
fn holds(rows: &[String], file: u32, place: u32) -> bool {
    let node = format!(
        "{:?}",
        Value::Node(ryo_relations::domain::relations::fixtures::node_id(
            FileId(file),
            ryo_relations::domain::relations::types::Position(place),
        ))
    );
    rows.iter().any(|row| row.contains(&node))
}

/// Whether some row pairs the first functions of the first two files, whose spans are equal.
fn pairs_two_files(rows: &[String]) -> bool {
    let first = |file: u32| {
        format!(
            "{:?}",
            Value::Node(ryo_relations::domain::relations::fixtures::node_id(
                FileId(file),
                ryo_relations::domain::relations::types::Position(0),
            ))
        )
    };
    rows.iter()
        .any(|row| row.contains(&first(0)) && row.contains(&first(1)))
}

#[test]
fn regrouping_a_join_on_spans_alone_leaves_it_after_the_files_and_keeps_the_pairs_of_two_files() {
    let files = files();
    let (steps, emitted) = chained(ColumnIndex(2));
    let before = rows(steps.clone(), emitted, &files);
    assert!(
        pairs_two_files(&before),
        "equal spans of two files meet after the files: {before:?}"
    );
    let (after, after_emitted) = regrouped(steps.clone(), emitted);
    assert_eq!(
        after, steps,
        "a span states no file, so the join stays after the files"
    );
    assert_eq!(rows(after, after_emitted, &files), before);
}

#[test]
fn regrouping_a_join_on_the_file_makes_it_in_each_file_with_the_same_rows() {
    let files = files();
    let (steps, emitted) = chained(ColumnIndex(1));
    let before = rows(steps.clone(), emitted, &files);
    assert!(!before.is_empty());
    assert!(
        !pairs_two_files(&before),
        "a file key pairs functions of one file"
    );
    let (after, after_emitted) = regrouped(steps.clone(), emitted);
    assert_ne!(
        after, steps,
        "the file key moves the join of the per-file inputs into each file"
    );
    assert_eq!(rows(after, after_emitted, &files), before);
}

/// `Join(gathered, functions on key)`: the nodes or files of the functions named `main` gathered after the files, met with every
/// function (id, file, span) on that column.
fn looked_up(key: ColumnIndex, local: ColumnIndex) -> (Vec<PlanStep>, StepId) {
    (
        vec![
            named_main(&[key]),
            gathered(StepId(0), ColumnIndex(0)),
            functions_kept(&[ID, FILE, SPAN]),
            after_the_files(join(
                StepId(1),
                StepId(2),
                (ColumnIndex(0), local),
                vec![
                    column(Side::Right, ColumnIndex(0)),
                    column(Side::Right, ColumnIndex(1)),
                    column(Side::Right, ColumnIndex(2)),
                ],
            )),
        ],
        StepId(3),
    )
}

#[test]
fn narrowing_by_the_nodes_or_files_the_other_input_holds_keeps_the_rows() {
    let files = files();
    for (key, local) in [(ID, ColumnIndex(0)), (FILE, ColumnIndex(1))] {
        let (steps, emitted) = looked_up(key, local);
        let before = rows(steps.clone(), emitted, &files);
        assert!(!before.is_empty());
        let (after, after_emitted) = narrowed(steps.clone(), emitted);
        assert_ne!(
            after, steps,
            "a node or a file key narrows the per-file input"
        );
        assert_eq!(rows(after, after_emitted, &files), before);
    }
}

#[test]
fn narrowing_leaves_a_join_on_spans_as_it_is() {
    let files = files();
    let (steps, emitted) = looked_up(SPAN, ColumnIndex(2));
    let before = rows(steps.clone(), emitted, &files);
    assert!(
        holds(&before, 2, 0),
        "the spans of `main` functions of two files meet the function of a third file that has none: {before:?}"
    );
    let (after, after_emitted) = narrowed(steps.clone(), emitted);
    assert_eq!(
        after, steps,
        "a span states no file, so nothing is cut in each file"
    );
    assert_eq!(rows(after, after_emitted, &files), before);
}

/// Every pair of functions of one file, joined on the file in each file, and the pairs whose second function is named `main`.
fn filtered_pairs() -> (Vec<PlanStep>, StepId) {
    let kept = [ID, NAME, FILE];
    (
        vec![
            functions_kept(&kept),
            functions_kept(&kept),
            in_each_file(join(
                StepId(0),
                StepId(1),
                (ColumnIndex(2), ColumnIndex(2)),
                vec![
                    column(Side::Left, ColumnIndex(0)),
                    column(Side::Right, ColumnIndex(0)),
                    column(Side::Right, ColumnIndex(1)),
                ],
            )),
            in_each_file(Operation::Select {
                from: Input::Step(StepId(2)),
                comparisons: vec![PlanComparison {
                    test: Test::Equal,
                    left: Operand::Column(ColumnIndex(2)),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText("main".into()))),
                    origin: ComparisonOrigin::Unrecorded,
                }],
                kept: vec![ColumnIndex(0), ColumnIndex(1)],
            }),
        ],
        StepId(3),
    )
}

#[test]
fn pushing_a_filter_into_a_join_on_the_file_keeps_the_rows() {
    let files = files();
    let (steps, emitted) = filtered_pairs();
    let before = rows(steps.clone(), emitted, &files);
    assert!(!before.is_empty());
    let (after, after_emitted) = selections_pushed(steps.clone(), emitted);
    assert_ne!(
        after, steps,
        "the filter of the second function moves onto its input"
    );
    assert_eq!(rows(after, after_emitted, &files), before);
}

#[test]
fn the_proofs_name_the_function_relations_columns_by_place() {
    use ryo_relations::domain::relations::{service::kinds, types::CellKind};
    assert_eq!(
        kinds(Relation::Function),
        [
            CellKind::Node,
            CellKind::Symbol,
            CellKind::File,
            CellKind::Span
        ]
    );
}
