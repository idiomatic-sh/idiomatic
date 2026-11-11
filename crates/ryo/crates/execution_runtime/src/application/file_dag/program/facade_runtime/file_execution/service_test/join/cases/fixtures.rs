//! The support of the `cases` cases.
use super::super::super::super::*;
use super::super::*;
#[cfg(test)]
use fixtures::execution::*;
#[cfg(test)]
use fixtures::provider::*;
use ryo_execution_domain::domain::table_cells::types::{TreeForm, TreeRows};
use ryo_language::domain::builders::service::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::symbols::types::Symbols;
use ryo_language::domain::{
    completion::service::{completion, name_kind},
    elements::types::Elements,
    kind_index::service::index_kinds,
    symbols::types::ByteSlice,
};
use ryo_relations::domain::relations::{
    service::node_id,
    types::{CellKind, FileId, Integer, Offset, Position, Span, Value as Cell},
};
use ryo_relations::domain::{relations::types::KindBits, terms::types::NameText};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Side},
    rows::types::Table,
};
use ryo_values::domain::{
    plan_parts::types::{LiteralText, Operand, PlanComparison, QueryLiteral, Test},
    rows::service::push_row,
};

pub(crate) fn node(at: u32) -> Cell {
    Cell::Node(node_id(FileId(0), Position(at)))
}

pub(crate) fn stretch(start: u32, end: u32) -> Cell {
    Cell::Span(Span {
        start: Offset(start),
        end: Offset(end),
    })
}

pub(crate) fn table_of(kinds: &[CellKind], rows: &[&[Cell]]) -> Table {
    let mut table = Table {
        kinds: kinds.to_vec(),
        ..Table::default()
    };
    for row in rows {
        push_row(&mut table, row);
    }
    table
}

pub(crate) fn wide_tree(tokens: u32, named: bool) -> (Elements, Symbols) {
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
    // Every actual tree source has a complete vocabulary. `named` selects the
    // branch spelling used by the kind-filter controls; it never means uncompleted syntax.
    for (bits, name) in [
        (1, "file"),
        (2, "word"),
        (3, if named { "inner" } else { "branch" }),
    ] {
        name_kind(
            &mut completion(&mut elements, &mut symbols, ByteSlice(b"abcd"), FileId(0)),
            KindBits(bits),
            NameText(name),
        );
    }
    index_kinds(&mut elements);
    (elements, symbols)
}

pub(crate) fn tree_rows(elements: &Elements, form: TreeForm) -> Rows<'_> {
    Rows::Tree(TreeRows {
        elements,
        file: FileId(0),
        form,
    })
}

pub(in super::super) fn text_of(place: u32, text: &str) -> PlanComparison {
    PlanComparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(place)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    }
}

pub(in super::super) fn agreed(
    made: Result<Table, OperatingError>,
    expected: &[Cell],
) -> Result<usize, OperatingError> {
    Ok(usize::from(made?.cells == expected))
}

/// Joins nodes of the file with the edges of its tree on the column of the edges, answering the parents found.
pub(in super::super) fn joins_the_tree(starts: &[u32], on: u32) -> Result<Table, OperatingError> {
    let (elements, mut symbols) = wide_tree(1, false);
    let mut left = table_of(&[CellKind::Node], &[]);
    for start in starts {
        push_row(&mut left, &[node(*start)]);
    }
    let output = [(Side::Right, 0), (Side::Right, 1)];
    let operation = operation(&[0], &[on], &output, Vec::new(), &[]);
    let inputs = [Rows::Computed(&left), tree_rows(&elements, TreeForm::Edges)];
    made(&operation, inputs, &mut symbols, &[])
}

pub(in super::super) fn a_node_joined_on_the_child_of_the_tree_finds_its_parent()
-> Result<usize, OperatingError> {
    let parents = [node(2), node(3), node(0), node(1)];
    agreed(joins_the_tree(&[3, 1], 1), &parents)
}

pub(in super::super) fn a_node_joined_on_the_parent_of_the_tree_finds_its_children()
-> Result<usize, OperatingError> {
    let children = [node(0), node(1), node(0), node(2)];
    agreed(joins_the_tree(&[0], 0), &children)
}

pub(in super::super) fn the_root_has_no_parent_and_a_leaf_no_children()
-> Result<usize, OperatingError> {
    let (root, leaf) = (joins_the_tree(&[0], 1)?, joins_the_tree(&[3], 0)?);
    Ok(usize::from(root.cells.is_empty() && leaf.cells.is_empty()))
}

/// Joins nodes with the elements on their ids, keeping those whose text is `b`: answers the node and the span.
pub(in super::super) fn only_the_elements_a_filter_admits_are_joined_by_their_ids()
-> Result<usize, OperatingError> {
    let (elements, mut symbols) = wide_tree(1, false);
    let left = table_of(&[CellKind::Node], &[&[node(1)], &[node(3)], &[node(2)]]);
    let output = [(Side::Left, 0), (Side::Right, 1)];
    let operation = operation(&[0], &[0], &output, vec![text_of(2, "b")], &[0, 4]);
    let inputs = [
        Rows::Computed(&left),
        tree_rows(&elements, TreeForm::Elements),
    ];
    let joined = made(&operation, inputs, &mut symbols, b"abcd");
    agreed(joined, &[node(3), stretch(1, 2)])
}

/// Joins the root with the edges whose child is the second of its parent's, child first: answers the children.
pub(in super::super) fn only_the_edges_a_filter_admits_are_joined() -> Result<usize, OperatingError>
{
    let (elements, mut symbols) = wide_tree(1, false);
    let left = table_of(&[CellKind::Node], &[&[node(0)]]);
    let second = PlanComparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(2)),
        right: Operand::Literal(QueryLiteral::Number(Integer(1))),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    };
    let operation = operation(&[0], &[1], &[(Side::Right, 0)], vec![second], &[1, 0]);
    let inputs = [Rows::Computed(&left), tree_rows(&elements, TreeForm::Edges)];
    agreed(made(&operation, inputs, &mut symbols, b"abcd"), &[node(2)])
}

/// Joins every edge of a wide tree with the elements of a kind, by the child of the edge: answers the parent and the child.
pub(in super::super) fn the_edges_of_the_children_of_a_kind_are_found_by_the_kind()
-> Result<usize, OperatingError> {
    let (elements, mut symbols) = wide_tree(40, true);
    let output = [(Side::Left, 0), (Side::Right, 0)];
    let operation = operation(&[1], &[0], &output, vec![text_of(1, "inner")], &[]);
    let inputs = [
        tree_rows(&elements, TreeForm::Edges),
        tree_rows(&elements, TreeForm::Elements),
    ];
    let joined = made(&operation, inputs, &mut symbols, b"abcd");
    agreed(joined, &[node(0), node(41)])
}

/// Joins the edges of a wide tree with a few rows naming nodes, by the child of the edge: answers the children, in the order of the edges.
pub(in super::super) fn a_few_rows_leave_in_the_order_of_the_edges() -> Result<usize, OperatingError>
{
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
    let operation = operation(&[1], &[0], &output, Vec::new(), &[]);
    let inputs = [
        tree_rows(&elements, TreeForm::Edges),
        Rows::Computed(&right),
    ];
    let joined = made(&operation, inputs, &mut symbols, b"abcd");
    let expected = [node(5), number(1), node(5), number(3), node(41), number(2)];
    agreed(joined, &expected)
}

/// Joins the stretch of the node at 41 of a wide tree, on the stretch alone, with the elements covering it.
pub(in super::super) fn the_elements_covering_a_stretch_are_found_by_it_alone()
-> Result<usize, OperatingError> {
    let (elements, mut symbols) = wide_tree(40, true);
    let left = table_of(&[CellKind::Span], &[&[stretch(1, 2)]]);
    let operation = operation(&[0], &[4], &[(Side::Right, 0)], Vec::new(), &[]);
    let inputs = [
        Rows::Computed(&left),
        tree_rows(&elements, TreeForm::Elements),
    ];
    let joined = made(&operation, inputs, &mut symbols, b"abcd");
    agreed(joined, &[node(41), node(42)])
}
