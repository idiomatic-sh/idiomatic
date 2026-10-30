//! The `join` item's cases.

use ryo_language::domain::completion::service::{completion, name_kind};
use ryo_language::domain::elements::service::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::kind_index::service::index_kinds;
use ryo_language::domain::relations::service::node_id;
use ryo_language::domain::relations::types::{
    CellKind, FileId, Integer, KindBits, Offset, Position, Relation, Span, Value as Cell,
};
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_language::domain::terms::types::NameText;
use ryo_values::domain::plans::types::QueryOperator;

use super::super::interface::OperatorsItem as _;
use super::item::JoinItem;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::table_cells::types::{TreeForm, TreeRows};
use crate::domain::table_model::service::push_row;
use crate::domain::table_model::types::{Rows, Table};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Comparison, Input as PlanInput, JoinColumn, LiteralText, Operand, QueryLiteral,
    Side, Test,
};
use ryo_values::domain::plans::types::Operation;

/// How many rows came out, or 1 when every expectation of the scenario held.
type Value = usize;
type Failure = OperatingError;
type Input = fn() -> Result<Value, OperatingError>;

const FROM: PlanInput = PlanInput::Relation(Relation::Function);

fn node(position: u32) -> Cell {
    Cell::Node(node_id(FileId(0), Position(position)))
}

fn stretch(start: u32, end: u32) -> Cell {
    Cell::Span(Span {
        start: Offset(start),
        end: Offset(end),
    })
}

fn table_of(kinds: &[CellKind], rows: &[&[Cell]]) -> Table {
    let mut table = Table {
        kinds: kinds.to_vec(),
        ..Table::default()
    };
    for row in rows {
        push_row(&mut table, row);
    }
    table
}

/// A join of `left_on` with `right_on`, answering the `output`, keeping what the `filter` admits, the right read through the `view`.
fn plan(
    left_on: &[u32],
    right_on: &[u32],
    output: &[(Side, u32)],
    filter: Vec<Comparison>,
    view: &[u32],
) -> Operation {
    let columns = |places: &[u32]| places.iter().map(|place| ColumnIndex(*place)).collect();
    Operation::Join {
        left: FROM,
        right: FROM,
        left_on: columns(left_on),
        right_on: columns(right_on),
        output: output
            .iter()
            .map(|(side, place)| JoinColumn {
                side: *side,
                column: ColumnIndex(*place),
            })
            .collect(),
        filter,
        view: columns(view),
    }
}

fn made(
    operation: &Operation,
    rows: &[Rows<'_>],
    symbols: &mut Symbols,
    source: &[u8],
) -> Result<Table, OperatingError> {
    let mut out = Table::default();
    let mut context = OperatorContext {
        bindings: &[],
        symbols,
        source: SourceView(source),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    JoinItem.compute(operation, rows, &mut context, &mut out)?;
    Ok(out)
}

/// A root holding `tokens` tokens and a node with a token (its kind named when `named`); with one token, positions 0 to 3.
fn wide_tree(tokens: u32, named: bool) -> (Elements, Symbols) {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    {
        let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"abcd"));
        enter_node(&mut builder, KindBits(1), Offset(0));
        for _ in 0..tokens {
            token(&mut builder, KindBits(2), Offset(0), Offset(1));
        }
        enter_node(&mut builder, KindBits(3), Offset(1));
        token(&mut builder, KindBits(2), Offset(1), Offset(2));
        leave_node(&mut builder, Offset(2));
        leave_node(&mut builder, Offset(2));
    }
    if named {
        name_kind(
            &mut completion(&mut elements, &mut symbols, ByteSlice(b"abcd"), FileId(0)),
            KindBits(3),
            NameText("inner"),
        );
        index_kinds(&mut elements);
    }
    (elements, symbols)
}

fn tree_rows(elements: &Elements, form: TreeForm) -> Rows<'_> {
    Rows::Tree(TreeRows {
        elements,
        file: FileId(0),
        form,
    })
}

fn text_of(place: u32, text: &str) -> Comparison {
    Comparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(place)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
    }
}

fn agreed(made: Result<Table, OperatingError>, expected: &[Cell]) -> Result<Value, OperatingError> {
    Ok(Value::from(made?.cells == expected))
}

/// Joins nodes of the file with the edges of its tree on the column of the edges, answering the parents found.
fn joins_the_tree(starts: &[u32], on: u32) -> Result<Table, OperatingError> {
    let (elements, mut symbols) = wide_tree(1, false);
    let mut left = table_of(&[CellKind::Node], &[]);
    for start in starts {
        push_row(&mut left, &[node(*start)]);
    }
    let output = [(Side::Right, 0), (Side::Right, 1)];
    let operation = plan(&[0], &[on], &output, Vec::new(), &[]);
    let inputs = [Rows::Computed(&left), tree_rows(&elements, TreeForm::Edges)];
    made(&operation, &inputs, &mut symbols, &[])
}

fn a_node_joined_on_the_child_of_the_tree_finds_its_parent() -> Result<Value, OperatingError> {
    let parents = [node(2), node(3), node(0), node(1)];
    agreed(joins_the_tree(&[3, 1], 1), &parents)
}

fn a_node_joined_on_the_parent_of_the_tree_finds_its_children() -> Result<Value, OperatingError> {
    let children = [node(0), node(1), node(0), node(2)];
    agreed(joins_the_tree(&[0], 0), &children)
}

fn the_root_has_no_parent_and_a_leaf_no_children() -> Result<Value, OperatingError> {
    let (root, leaf) = (joins_the_tree(&[0], 1)?, joins_the_tree(&[3], 0)?);
    Ok(Value::from(root.cells.is_empty() && leaf.cells.is_empty()))
}

/// Joins nodes with the elements on their ids, keeping those whose text is `b`: answers the node and the span.
fn only_the_elements_a_filter_admits_are_joined_by_their_ids() -> Result<Value, OperatingError> {
    let (elements, mut symbols) = wide_tree(1, false);
    let left = table_of(&[CellKind::Node], &[&[node(1)], &[node(3)], &[node(2)]]);
    let output = [(Side::Left, 0), (Side::Right, 1)];
    let operation = plan(&[0], &[0], &output, vec![text_of(2, "b")], &[0, 4]);
    let inputs = [
        Rows::Computed(&left),
        tree_rows(&elements, TreeForm::Elements),
    ];
    let joined = made(&operation, &inputs, &mut symbols, b"abcd");
    agreed(joined, &[node(3), stretch(1, 2)])
}

/// Joins the root with the edges whose child is the second of its parent's, child first: answers the children.
fn only_the_edges_a_filter_admits_are_joined() -> Result<Value, OperatingError> {
    let (elements, mut symbols) = wide_tree(1, false);
    let left = table_of(&[CellKind::Node], &[&[node(0)]]);
    let second = Comparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(2)),
        right: Operand::Literal(QueryLiteral::Number(Integer(1))),
    };
    let operation = plan(&[0], &[1], &[(Side::Right, 0)], vec![second], &[1, 0]);
    let inputs = [Rows::Computed(&left), tree_rows(&elements, TreeForm::Edges)];
    agreed(made(&operation, &inputs, &mut symbols, b"abcd"), &[node(2)])
}

/// Joins every edge of a wide tree with the elements of a kind, by the child of the edge: answers the parent and the child.
fn the_edges_of_the_children_of_a_kind_are_found_by_the_kind() -> Result<Value, OperatingError> {
    let (elements, mut symbols) = wide_tree(40, true);
    let output = [(Side::Left, 0), (Side::Right, 0)];
    let operation = plan(&[1], &[0], &output, vec![text_of(1, "inner")], &[]);
    let inputs = [
        tree_rows(&elements, TreeForm::Edges),
        tree_rows(&elements, TreeForm::Elements),
    ];
    let joined = made(&operation, &inputs, &mut symbols, b"abcd");
    agreed(joined, &[node(0), node(41)])
}

/// Joins the edges of a wide tree with a few rows naming nodes, by the child of the edge: answers the children, in the order of the edges.
fn a_few_rows_leave_in_the_order_of_the_edges() -> Result<Value, OperatingError> {
    let (elements, mut symbols) = wide_tree(40, true);
    let number = |value| Cell::Number(Integer(value));
    let right = table_of(
        &[CellKind::Node, CellKind::Number],
        &[
            &[node(41), number(2)],
            &[node(5), number(1)],
            &[node(5), number(3)],
        ],
    );
    let output = [(Side::Left, 1), (Side::Right, 1)];
    let operation = plan(&[1], &[0], &output, Vec::new(), &[]);
    let inputs = [
        tree_rows(&elements, TreeForm::Edges),
        Rows::Computed(&right),
    ];
    let joined = made(&operation, &inputs, &mut symbols, b"abcd");
    let expected = [node(5), number(1), node(5), number(3), node(41), number(2)];
    agreed(joined, &expected)
}

/// Joins the stretch of the node at 41 of a wide tree, on the stretch alone, with the elements covering it.
fn the_elements_covering_a_stretch_are_found_by_it_alone() -> Result<Value, OperatingError> {
    let (elements, mut symbols) = wide_tree(40, true);
    let left = table_of(&[CellKind::Span], &[&[stretch(1, 2)]]);
    let operation = plan(&[0], &[4], &[(Side::Right, 0)], Vec::new(), &[]);
    let inputs = [
        Rows::Computed(&left),
        tree_rows(&elements, TreeForm::Elements),
    ];
    let joined = made(&operation, &inputs, &mut symbols, b"abcd");
    agreed(joined, &[node(41), node(42)])
}

/// Computes a join of two tables of numbers (the second is left out when it is `None`), answering how many rows came out.
fn counted(left: &Table, right: Option<&Table>) -> Result<Value, OperatingError> {
    let output = [(Side::Left, 1), (Side::Right, 1)];
    let operation = plan(&[0], &[0], &output, Vec::new(), &[]);
    let rows: Vec<Rows<'_>> = [Some(left), right]
        .into_iter()
        .flatten()
        .map(Rows::Computed)
        .collect();
    made(&operation, &rows, &mut Symbols::default(), &[]).map(|joined| joined.count.0)
}

fn joins_on_equal_keys() -> Result<Value, OperatingError> {
    let pairs = |rows: &[[i64; 2]]| {
        let mut table = table_of(&[CellKind::Number, CellKind::Number], &[]);
        for row in rows {
            push_row(&mut table, &row.map(|number| Cell::Number(Integer(number))));
        }
        table
    };
    let (left, right) = (
        pairs(&[[1, 10], [2, 20]]),
        pairs(&[[1, 100], [1, 200], [3, 300]]),
    );
    counted(&left, Some(&right))
}

fn joins_without_the_right_input() -> Result<Value, OperatingError> {
    let left = table_of(&[CellKind::Number], &[&[Cell::Number(Integer(1))]]);
    counted(&left, None)
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("equal_keys_combine"),
        word: Word::Succeeds,
        input: || joins_on_equal_keys,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a_node_joined_on_the_child_of_the_tree_finds_its_parent"),
        word: Word::Succeeds,
        input: || a_node_joined_on_the_child_of_the_tree_finds_its_parent,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_node_joined_on_the_parent_of_the_tree_finds_its_children"),
        word: Word::Succeeds,
        input: || a_node_joined_on_the_parent_of_the_tree_finds_its_children,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_root_has_no_parent_and_a_leaf_no_children"),
        word: Word::Succeeds,
        input: || the_root_has_no_parent_and_a_leaf_no_children,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("only_the_elements_a_filter_admits_are_joined_by_their_ids"),
        word: Word::Succeeds,
        input: || only_the_elements_a_filter_admits_are_joined_by_their_ids,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("only_the_edges_a_filter_admits_are_joined_in_the_view_they_are_asked_in"),
        word: Word::Succeeds,
        input: || only_the_edges_a_filter_admits_are_joined,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_edges_of_the_children_of_a_kind_are_found_by_the_kind"),
        word: Word::Succeeds,
        input: || the_edges_of_the_children_of_a_kind_are_found_by_the_kind,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_elements_covering_a_stretch_are_found_by_it_alone"),
        word: Word::Succeeds,
        input: || the_elements_covering_a_stretch_are_found_by_it_alone,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_few_rows_find_their_edges_and_leave_in_the_order_of_the_edges"),
        word: Word::Succeeds,
        input: || a_few_rows_leave_in_the_order_of_the_edges,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_missing_input_is_refused"),
        word: Word::Refuses,
        input: || joins_without_the_right_input,
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
    super::super::fixtures::contract::check(QueryOperator::Join);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
