//! The `axis` item's cases.

use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::relations::types::{CellKind, Relation, Value as Cell};
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::plans::types::QueryOperator;

use ryo_language::domain::symbols::types::Symbols;

use super::super::interface::OperatorsItem as _;
use super::item::AxisItem;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::table_cells::types::{TreeForm, TreeRows};
use crate::domain::table_model::service::push_row;
use crate::domain::table_model::types::{ParameterBinding, Rows, Table};
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_language::domain::elements::service::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::relations::service::node_id;
use ryo_language::domain::relations::types::Integer;
use ryo_language::domain::relations::types::{FileId, KindBits, Offset, Position};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Comparison;
use ryo_values::domain::plan_parts::types::Direction;
use ryo_values::domain::plan_parts::types::Input as Source;
use ryo_values::domain::plan_parts::types::JoinColumn;
use ryo_values::domain::plan_parts::types::Operand;
use ryo_values::domain::plan_parts::types::QueryLiteral;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plan_parts::types::Test;
use ryo_values::domain::plans::types::Operation;

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
    let mut symbols = Symbols::default();
    let rows: Vec<Rows<'_>> = inputs.iter().map(|table| Rows::Computed(table)).collect();
    let mut out = Table::default();
    let mut context = OperatorContext {
        stored_indexes: None,
        bindings,
        symbols: &mut symbols,
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    AxisItem
        .compute(operation, &rows, &mut context, &mut out)
        .map(|()| out.count.0)
}

/// The tree of a file: a root holding a token and a node, which holds a token. Positions 0 to 3.
fn tree() -> (Elements, Symbols) {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"abcd"));
    enter_node(&mut builder, KindBits(1), Offset(0));
    token(&mut builder, KindBits(2), Offset(0), Offset(1));
    enter_node(&mut builder, KindBits(3), Offset(1));
    token(&mut builder, KindBits(2), Offset(1), Offset(2));
    leave_node(&mut builder, Offset(2));
    leave_node(&mut builder, Offset(2));
    (elements, symbols)
}

fn node(position: u32) -> Cell {
    Cell::Node(node_id(FileId(0), Position(position)))
}

/// What the third input of a walk over the tree is: the nodes of a table, or the elements of the tree.
enum Domain {
    /// A table of nodes.
    Nodes(Table),
    /// The elements of the tree.
    Elements,
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
fn axis(direction: Direction, filter: Vec<Comparison>, output: Vec<JoinColumn>) -> Operation {
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

/// What the item makes of the operation over the tree, from the nodes in `starts`.
fn over_the_tree(
    operation: &Operation,
    starts: &Table,
    domain: &Domain,
) -> Result<Table, OperatingError> {
    let (elements, mut symbols) = tree();
    let (source, form) = match domain {
        Domain::Nodes(_) => (&b""[..], TreeForm::Edges),
        Domain::Elements => (&b"abcd"[..], TreeForm::Elements),
    };
    let rows = [
        Rows::Computed(starts),
        Rows::Tree(TreeRows {
            elements: &elements,
            file: FileId(0),
            form: TreeForm::Edges,
        }),
        match domain {
            Domain::Nodes(table) => Rows::Computed(table),
            Domain::Elements => Rows::Tree(TreeRows {
                elements: &elements,
                file: FileId(0),
                form,
            }),
        },
    ];
    let mut out = Table::default();
    let mut context = OperatorContext {
        stored_indexes: None,
        bindings: &[],
        symbols: &mut symbols,
        source: SourceView(source),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    AxisItem.compute(operation, &rows, &mut context, &mut out)?;
    Ok(out)
}

/// The entities found walking `direction` from the node at `start` in the tree, among all the nodes of the file.
fn walks_the_tree(direction: Direction, start: u32) -> Result<Vec<Cell>, OperatingError> {
    let out = over_the_tree(
        &axis(direction, Vec::new(), Vec::new()),
        &node_table([start]),
        &Domain::Nodes(node_table(0..4)),
    )?;
    let mut found: Vec<Cell> = out.cells.chunks(2).map(|row| row[1]).collect();
    found.sort();
    Ok(found)
}

fn descendants_in_the_tree_are_everything_below() -> Result<Value, OperatingError> {
    Ok(usize::from(
        walks_the_tree(Direction::Descendant, 0)? == [node(1), node(2), node(3)]
            && walks_the_tree(Direction::Descendant, 2)? == [node(3)]
            && walks_the_tree(Direction::Descendant, 3)?.is_empty(),
    ))
}

fn ancestors_in_the_tree_are_the_parents_up_to_the_root() -> Result<Value, OperatingError> {
    Ok(usize::from(
        walks_the_tree(Direction::Ancestor, 3)? == [node(0), node(2)]
            && walks_the_tree(Direction::Ancestor, 0)?.is_empty(),
    ))
}

fn the_nearest_ancestor_in_the_tree_is_the_parent() -> Result<Value, OperatingError> {
    Ok(usize::from(
        walks_the_tree(Direction::NearestAncestor, 3)? == [node(2)],
    ))
}

fn the_parent_and_the_children_in_the_tree() -> Result<Value, OperatingError> {
    Ok(usize::from(
        walks_the_tree(Direction::Parent, 3)? == [node(2)]
            && walks_the_tree(Direction::Child, 0)? == [node(1), node(2)]
            && walks_the_tree(Direction::Parent, 0)?.is_empty(),
    ))
}

fn the_siblings_in_the_tree_are_the_children_beside_under_the_same_parent()
-> Result<Value, OperatingError> {
    Ok(usize::from(
        walks_the_tree(Direction::PreviousSibling, 2)? == [node(1)]
            && walks_the_tree(Direction::NextSibling, 1)? == [node(2)]
            && walks_the_tree(Direction::PreviousSibling, 1)?.is_empty()
            && walks_the_tree(Direction::NextSibling, 2)?.is_empty()
            && walks_the_tree(Direction::NextSibling, 3)?.is_empty()
            && walks_the_tree(Direction::PreviousSibling, 0)?.is_empty(),
    ))
}

/// The descendants of the root among the elements of the tree that a filter admits, read from the tree.
fn walks_among_the_filtered_elements(filter: Vec<Comparison>) -> Result<Vec<Cell>, OperatingError> {
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
    let text_is_b = Comparison {
        test: Test::Equal,
        left: Operand::Column(column(2)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText("b".into()))),
        origin: None,
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
    compute(&operation, &[&numbers(&[&[1]]), &numbers(&[&[1, 2]])], &[])
}

/// A sparse computed-edge walk retains payloads, crosses non-domain nodes and
/// visits no unrelated component, including equal local positions in another file.
fn sparse_computed_walk_work_is_rooted() -> Result<Value, OperatingError> {
    let mut scratch = OperatorScratch::default();
    let mut symbols = Symbols::default();
    let mut memo = Vec::new();
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
            let inputs = [
                Rows::Computed(&starts),
                Rows::Computed(&edges),
                Rows::Computed(&domain),
            ];
            let mut out = Table::default();
            let mut context = OperatorContext {
                stored_indexes: None,
                bindings: &[],
                symbols: &mut symbols,
                source: SourceView(&[]),
                paths: &[],
                arenas: &[],
                memo: &mut memo,
                scratch: &mut scratch,
            };
            AxisItem.compute(
                &axis(Direction::Descendant, Vec::new(), Vec::new()),
                &inputs,
                &mut context,
                &mut out,
            )?;
            let expected_targets: &[u32] = if cycle { &[3, 2, 1] } else { &[3, 2] };
            let mut expected = Vec::new();
            for payload in [11, 22] {
                for target in expected_targets {
                    expected.extend([node(1), Cell::Number(Integer(payload)), node(*target)]);
                }
            }
            if out.cells != expected
                || out.count.0 != 2 * expected_targets.len()
                || context.scratch.walk.visited.len() != if cycle { 4 } else { 3 }
            {
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
        name: CaseName("descendants_in_the_tree_are_everything_below"),
        word: Word::Succeeds,
        input: || descendants_in_the_tree_are_everything_below,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("ancestors_in_the_tree_are_the_parents_up_to_the_root"),
        word: Word::Succeeds,
        input: || ancestors_in_the_tree_are_the_parents_up_to_the_root,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_nearest_ancestor_in_the_tree_is_the_parent"),
        word: Word::Succeeds,
        input: || the_nearest_ancestor_in_the_tree_is_the_parent,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_parent_and_the_children_in_the_tree"),
        word: Word::Succeeds,
        input: || the_parent_and_the_children_in_the_tree,
        expect: Expect::Value(1),
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
        name: CaseName("the_siblings_in_the_tree_are_the_children_beside_under_the_same_parent"),
        word: Word::Succeeds,
        input: || the_siblings_in_the_tree_are_the_children_beside_under_the_same_parent,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_missing_input_is_refused"),
        word: Word::Refuses,
        input: || walks_with_two_inputs,
        expect: Expect::Refusal(|error| {
            selects(matches!(error, OperatingError::MissingInput { .. }))
        }),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    super::super::fixtures::contract::check(QueryOperator::Axis);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
