//! Axis rows and native tree traversal through real compiled file owners.
use super::super::super::fixtures;
use super::super::axis_tree::test::{Domain, over_the_tree};
use ryo_values::selects;
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::Relation;
use ryo_relations::domain::relations::types::Value as Cell;
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_language::domain::symbols::types::Symbols;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::elements::types::Elements;
use ryo_relations::domain::relations::service::node_id;
use ryo_relations::domain::relations::types::Integer;
use ryo_relations::domain::relations::types::FileId;
use ryo_relations::domain::relations::types::Position;
use ryo_values::cases::Case;
use ryo_values::cases::CaseName;
use ryo_values::cases::Expect;
use ryo_values::cases::Word;
use ryo_values::cases::run;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Direction;
use ryo_values::domain::plan_parts::types::Input as Source;
use ryo_values::domain::plan_parts::types::JoinColumn;
use ryo_values::domain::plan_parts::types::Operand;
use ryo_values::domain::plan_parts::types::PlanComparison;
use ryo_values::domain::plan_parts::types::QueryLiteral;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plan_parts::types::Test;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::service::push_row;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::rows::types::Table;

type Value = usize;
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
    Ok(fixtures::requests::table_request(
        operation.clone(),
        inputs,
        bindings.to_vec(),
        &mut Symbols::default(),
    )?
    .count
    .0)
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
    Ok(usize::from(
        walks_among_the_filtered_elements(vec![text_is_b])? == [node(3)]
            && walks_among_the_filtered_elements(Vec::new())? == [node(1), node(2), node(3)],
    ))
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
    Ok(usize::from(
        // Of the root's children, the token at 1 and the node at 2, only the token at 3 is below them.
        walks_below_the_children(Vec::new())? == [vec![node(0), node(3)]]
            && walks_below_the_children(vec![right(0), left])? == [vec![node(3), node(0)]],
    ))
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
    .map(|kinds| kinds.len())
}
/// A sparse computed-edge walk retains payloads and crosses non-domain nodes,
/// including equal local positions in another file; the runtime proves its visits.
fn sparse_computed_walk_work_is_rooted() -> Result<Value, OperatingError> {
    let mut symbols = Symbols::default();
    for unrelated in [0_u32, 64, 4096] {
        for cycle in [false, true] {
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
            let expected_targets: &[u32] = if cycle { &[3, 2, 1] } else { &[3, 2] };
            let mut expected = Vec::new();
            for payload in [11, 22] {
                for target in expected_targets {
                    expected.extend([node(1), Cell::Number(Integer(payload)), node(*target)]);
                }
            }
            if out.cells != expected || out.count.0 != 2 * expected_targets.len() {
                return Ok(0);
            }
        }
    }
    Ok(1)
}
/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("sparse_computed_walk_work_is_rooted"),
        word: Word::Succeeds,
        input: || sparse_computed_walk_work_is_rooted,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("descendants_are_appended"),
        word: Word::Succeeds,
        input: || finds_the_descendants,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a_deep_walk_answers_what_is_below_the_children_with_the_columns_asked"),
        word: Word::Succeeds,
        input: || a_deep_walk_answers_what_is_below_the_children,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("only_the_elements_a_filter_of_the_domain_admits_answer"),
        word: Word::Succeeds,
        input: || only_the_elements_a_filter_admits_answer,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_missing_input_is_refused"),
        word: Word::Refuses,
        input: || walks_with_two_inputs,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, OperatingError::MissingInput { .. }))
        }),
    },];
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
