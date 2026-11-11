//! Axis rows and native tree traversal through real compiled file owners.
use super::super::super::fixtures;
use super::super::axis_tree::test::{Domain, over_the_tree};
use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::service::{node_file, node_id, node_position};
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::FileId;
use ryo_relations::domain::relations::types::Integer;
use ryo_relations::domain::relations::types::Position;
use ryo_relations::domain::relations::types::Relation;
use ryo_relations::domain::relations::types::Value as Cell;
use ryo_values::cases::Case;
use ryo_values::cases::CaseName;
use ryo_values::cases::Expect;
use ryo_values::cases::Word;
use ryo_values::cases::run;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Direction;
use ryo_values::domain::plan_parts::types::Input as Source;
use ryo_values::domain::plan_parts::types::JoinColumn;
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::plan_parts::types::Operand;
use ryo_values::domain::plan_parts::types::PlanComparison;
use ryo_values::domain::plan_parts::types::QueryLiteral;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plan_parts::types::Test;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::rows::service::push_row;
use ryo_values::domain::rows::types::Table;
use ryo_values::selects;
use std::borrow::Cow;

/// A cell a walk answered: a node by its position, a number, or another cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Seen {
    Node(u32),
    Number(i64),
    Other,
}
/// Each labelled walk's cells, in order.
type Value = Cow<'static, [(&'static str, Cow<'static, [Seen]>)]>;
type Failure = OperatingError;
/// A scenario: what a row computes with the item, answering how many rows came out.
type Input = fn() -> Result<Value, OperatingError>;
const FROM: Source = Source::Relation(Relation::Function);
fn column(index: u32) -> ColumnIndex {
    ColumnIndex(index)
}
/// A table of numbers, one row for each slice.
fn numbers(rows: &[&[i64]]) -> Table {
    let width = rows.first().map_or(1, |row| row.len());
    let mut table = Table {
        kinds: vec![CellKind::Number; width],
        ..Table::default()
    };
    for row in rows {
        push_row(
            &mut table,
            &row.iter()
                .map(|number| Cell::Number(Integer(*number)))
                .collect::<Vec<_>>(),
        );
    }
    table
}
fn compute(
    operation: &Operation,
    inputs: &[&Table],
    bindings: &[ParameterBinding],
) -> Result<Value, OperatingError> {
    let out = fixtures::requests::table_request(
        operation.clone(),
        inputs,
        bindings.to_vec(),
        &mut Symbols::default(),
    )?;
    Ok(vec![("cells", seen(&out.cells))].into())
}
fn node(position: u32) -> Cell {
    Cell::Node(node_id(FileId(0), Position(position)))
}
/// A table of one node column holding `starts`.
fn node_table(starts: impl IntoIterator<Item = u32>) -> Table {
    let mut table = Table {
        kinds: vec![CellKind::Node],
        ..Table::default()
    };
    starts
        .into_iter()
        .for_each(|position| push_row(&mut table, &[node(position)]));
    table
}
/// The axis operation that walks `direction` over the tree, from column 0 to column 1 of the edges, with the domain's filter and output.
fn axis(direction: Direction, filter: Vec<PlanComparison>, output: Vec<JoinColumn>) -> Operation {
    Operation::Axis {
        from: FROM,
        edges: FROM,
        domain: FROM,
        direction,
        id: column(0),
        parent: column(0),
        child: column(1),
        domain_id: column(0),
        domain_filter: filter,
        domain_view: Vec::new(),
        output,
    }
}
/// The descendants of the root among the elements of the tree that a filter admits, read from the tree.
fn walks_among_the_filtered_elements(
    filter: Vec<PlanComparison>,
) -> Result<Vec<Cell>, OperatingError> {
    let out = over_the_tree(
        &axis(Direction::Descendant, filter, Vec::new()),
        &node_table([0]),
        &Domain::Elements,
    )?;
    let mut found: Vec<Cell> = out.cells.chunks(2).map(|row| row[1]).collect();
    found.sort();
    Ok(found)
}
fn only_the_elements_a_filter_admits_answer() -> Result<Value, OperatingError> {
    let text_is_b = PlanComparison {
        test: Test::Equal,
        left: Operand::Column(column(2)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText("b".into()))),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    };
    Ok(vec![
        (
            "text b",
            seen(&walks_among_the_filtered_elements(vec![text_is_b])?),
        ),
        (
            "no filter",
            seen(&walks_among_the_filtered_elements(Vec::new())?),
        ),
    ]
    .into())
}
/// What a walk below the children of the root answers, as the rows `output` names.
fn walks_below_the_children(output: Vec<JoinColumn>) -> Result<Vec<Vec<Cell>>, OperatingError> {
    let out = over_the_tree(
        &axis(Direction::Deep, Vec::new(), output),
        &node_table([0]),
        &Domain::Elements,
    )?;
    let width = out.kinds.len();
    Ok(out.cells.chunks(width).map(<[Cell]>::to_vec).collect())
}
fn a_deep_walk_answers_what_is_below_the_children() -> Result<Value, OperatingError> {
    let left = JoinColumn {
        side: Side::Left,
        column: column(0),
    };
    let right = |at: u32| JoinColumn {
        side: Side::Right,
        column: column(at),
    };
    // Of the root's children, the token at 1 and the node at 2, only the token at 3 is below them.
    Ok(vec![
        (
            "default columns",
            seen(&walks_below_the_children(Vec::new())?.concat()),
        ),
        (
            "right then left",
            seen(&walks_below_the_children(vec![right(0), left])?.concat()),
        ),
    ]
    .into())
}
fn finds_the_descendants() -> Result<Value, OperatingError> {
    let operation = Operation::Axis {
        from: FROM,
        edges: FROM,
        domain: FROM,
        direction: Direction::Descendant,
        id: column(0),
        parent: column(0),
        child: column(1),
        domain_id: column(0),
        domain_filter: Vec::new(),
        domain_view: Vec::new(),
        output: Vec::new(),
    };
    let from = numbers(&[&[1]]);
    let edges = numbers(&[&[1, 2], &[2, 3]]);
    let domain = numbers(&[&[1], &[2], &[3]]);
    compute(&operation, &[&from, &edges, &domain], &[])
}
fn walks_with_two_inputs() -> Result<Value, OperatingError> {
    let operation = Operation::Axis {
        from: FROM,
        edges: FROM,
        domain: FROM,
        direction: Direction::Parent,
        id: column(0),
        parent: column(0),
        child: column(1),
        domain_id: column(0),
        domain_filter: Vec::new(),
        domain_view: Vec::new(),
        output: Vec::new(),
    };
    let kinds = [CellKind::Number];
    let edges = [CellKind::Number; 2];
    ryo_execution_domain::domain::operation_schema::service::schema_of(
        &operation,
        &[
            ryo_execution_domain::domain::operation_schema::types::InputShape {
                kinds: &kinds,
                tree: None,
            },
            ryo_execution_domain::domain::operation_schema::types::InputShape {
                kinds: &edges,
                tree: None,
            },
        ],
        &[],
    )
    .map(|kinds| vec![("columns", vec![Seen::Other; kinds.len()].into())].into())
}
/// A sparse computed-edge walk retains payloads and crosses non-domain nodes,
/// including equal local positions in another file; the runtime proves its visits.
fn sparse_computed_walk_work_is_rooted() -> Result<Value, OperatingError> {
    let mut symbols = Symbols::default();
    let mut walks = vec![];
    for (unrelated, cycle, label) in [
        (0_u32, false, "no unrelated edge, acyclic"),
        (0, true, "no unrelated edge, cyclic"),
        (64, false, "64 unrelated edges, acyclic"),
        (64, true, "64 unrelated edges, cyclic"),
        (4096, false, "4096 unrelated edges, acyclic"),
        (4096, true, "4096 unrelated edges, cyclic"),
    ] {
        let mut edges = Table {
            kinds: vec![CellKind::Node; 2],
            ..Table::default()
        };
        for [parent, child] in [[1, 4], [4, 2], [1, 3]] {
            push_row(&mut edges, &[node(parent), node(child)]);
        }
        if cycle {
            push_row(&mut edges, &[node(2), node(1)]);
        }
        for position in 1..=unrelated {
            push_row(
                &mut edges,
                &[
                    Cell::Node(node_id(FileId(1), Position(position))),
                    Cell::Node(node_id(FileId(1), Position(position + 1))),
                ],
            );
        }
        let mut starts = Table {
            kinds: vec![CellKind::Node, CellKind::Number],
            ..Table::default()
        };
        for payload in [11, 22] {
            push_row(&mut starts, &[node(1), Cell::Number(Integer(payload))]);
        }
        let domain = node_table([1, 2, 3]);
        let out = fixtures::requests::table_request(
            axis(Direction::Descendant, vec![], vec![]),
            &[&starts, &edges, &domain],
            vec![],
            &mut symbols,
        )?;
        walks.push((label, seen(&out.cells)));
    }
    Ok(walks.into())
}
/// What each cell is: a node of file 0 by its position, a number, or another cell.
fn seen(cells: &[Cell]) -> Cow<'static, [Seen]> {
    cells
        .iter()
        .map(|cell| match cell {
            Cell::Node(node) if node_file(*node) == FileId(0) => Seen::Node(node_position(*node).0),
            Cell::Number(number) => Seen::Number(number.0),
            Cell::Node(_)
            | Cell::Symbol(_)
            | Cell::Name(_)
            | Cell::File(_)
            | Cell::Span(_)
            | Cell::Flag(_) => Seen::Other,
        })
        .collect()
}
use Seen::{Node, Number};
/// The rows of a start at node 1, with payloads 11 and 22, reaching 3 then 2.
const ACYCLIC: &[Seen] = &[
    Node(1),
    Number(11),
    Node(3),
    Node(1),
    Number(11),
    Node(2),
    Node(1),
    Number(22),
    Node(3),
    Node(1),
    Number(22),
    Node(2),
];
/// The same over the cycle back to 1, which reaches 1 too.
const CYCLIC: &[Seen] = &[
    Node(1),
    Number(11),
    Node(3),
    Node(1),
    Number(11),
    Node(2),
    Node(1),
    Number(11),
    Node(1),
    Node(1),
    Number(22),
    Node(3),
    Node(1),
    Number(22),
    Node(2),
    Node(1),
    Number(22),
    Node(1),
];
const ROOTED: Value = Cow::Borrowed(&[
    ("no unrelated edge, acyclic", Cow::Borrowed(ACYCLIC)),
    ("no unrelated edge, cyclic", Cow::Borrowed(CYCLIC)),
    ("64 unrelated edges, acyclic", Cow::Borrowed(ACYCLIC)),
    ("64 unrelated edges, cyclic", Cow::Borrowed(CYCLIC)),
    ("4096 unrelated edges, acyclic", Cow::Borrowed(ACYCLIC)),
    ("4096 unrelated edges, cyclic", Cow::Borrowed(CYCLIC)),
]);
const APPENDED: Value = Cow::Borrowed(&[(
    "cells",
    Cow::Borrowed(&[Number(1), Number(2), Number(1), Number(3)]),
)]);
const BELOW: Value = Cow::Borrowed(&[
    ("default columns", Cow::Borrowed(&[Node(0), Node(3)])),
    ("right then left", Cow::Borrowed(&[Node(3), Node(0)])),
]);
const FILTERED: Value = Cow::Borrowed(&[
    ("text b", Cow::Borrowed(&[Node(3)])),
    ("no filter", Cow::Borrowed(&[Node(1), Node(2), Node(3)])),
]);
/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("sparse_computed_walk_work_is_rooted"),
        word: Word::Succeeds,
        input: || sparse_computed_walk_work_is_rooted,
        expect: Expect::Value(ROOTED),
    },
    Case {
        name: CaseName("descendants_are_appended"),
        word: Word::Succeeds,
        input: || finds_the_descendants,
        expect: Expect::Value(APPENDED),
    },
    Case {
        name: CaseName("a_deep_walk_answers_what_is_below_the_children_with_the_columns_asked"),
        word: Word::Succeeds,
        input: || a_deep_walk_answers_what_is_below_the_children,
        expect: Expect::Value(BELOW),
    },
    Case {
        name: CaseName("only_the_elements_a_filter_of_the_domain_admits_answer"),
        word: Word::Succeeds,
        input: || only_the_elements_a_filter_admits_answer,
        expect: Expect::Value(FILTERED),
    },
    Case {
        name: CaseName("a_missing_input_is_refused"),
        word: Word::Refuses,
        input: || walks_with_two_inputs,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, OperatingError::MissingInput { .. }))
        }),
    },
];
fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
