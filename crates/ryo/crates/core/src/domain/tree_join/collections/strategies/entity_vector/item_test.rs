//! The `entity_vector` item's cases: the elements a node names.

use ryo_language::domain::elements::service::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::relations::service::node_id;
use ryo_language::domain::relations::types::{FileId, KindBits, Offset, Position, Value as Cell};
use ryo_language::domain::symbols::types::ByteSlice;

use ryo_language::domain::symbols::types::Symbols;

use super::super::types::NodeColumns;
use super::item::EntityVectorItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_cells::types::{TreeForm, TreeRows};
use crate::domain::table_model::errors::TableModelError;
use crate::domain::tree_join::collections::strategies::interface::StrategiesItem as _;
use crate::domain::tree_join::collections::strategies::types::{Finding, Fitting};
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// What a row asks: the row on the left, which tree form is read, and by which columns.
type Input = fn() -> Result<[u32; 4], OperatingError>;
/// The positions found.
type Value = [u32; 4];
/// What pads a row of answers that found fewer than four.
const NONE: u32 = u32::MAX;

/// The answers padded to four.
fn four(found: Vec<u32>) -> [u32; 4] {
    let mut padded = [NONE; 4];
    for (slot, each) in padded.iter_mut().zip(found) {
        *slot = each;
    }
    padded
}
type Failure = OperatingError;

/// The tree of a file: a root holding a token and a node, which holds a token. Positions 0 to 3, the text `abcd`.
fn tree() -> Elements {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"abcd"));
    enter_node(&mut builder, KindBits(1), Offset(0));
    token(&mut builder, KindBits(2), Offset(0), Offset(1));
    enter_node(&mut builder, KindBits(3), Offset(1));
    token(&mut builder, KindBits(2), Offset(1), Offset(2));
    leave_node(&mut builder, Offset(2));
    leave_node(&mut builder, Offset(2));
    elements
}

fn found(
    row: &[Cell],
    form: TreeForm,
    columns: (u32, u32),
    file: u32,
) -> Result<[u32; 4], OperatingError> {
    let elements = tree();
    let rows = TreeRows {
        elements: &elements,
        file: FileId(file),
        form,
    };
    let mut found = Vec::new();
    EntityVectorItem::find(
        &rows,
        row,
        NodeColumns {
            left: ColumnIndex(columns.0),
            key: ColumnIndex(columns.1),
        },
        &mut found,
    )?;
    Ok(four(found.into_iter().map(|each| each.0).collect()))
}

fn node(position: u32, file: u32) -> Cell {
    Cell::Node(node_id(FileId(file), Position(position)))
}

fn a_node_names_its_element() -> Result<[u32; 4], OperatingError> {
    found(&[node(2, 0)], TreeForm::Elements, (0, 0), 0)
}

fn a_node_names_the_edge_that_leads_to_it() -> Result<[u32; 4], OperatingError> {
    found(&[node(2, 0)], TreeForm::Edges, (0, 1), 0)
}

fn the_root_is_led_to_by_no_edge() -> Result<[u32; 4], OperatingError> {
    found(&[node(0, 0)], TreeForm::Edges, (0, 1), 0)
}

fn a_parent_names_the_children_of_its_edges() -> Result<[u32; 4], OperatingError> {
    found(&[node(0, 0)], TreeForm::Edges, (0, 0), 0)
}

fn a_node_of_another_file_names_nothing() -> Result<[u32; 4], OperatingError> {
    found(&[node(2, 1)], TreeForm::Elements, (0, 0), 0)
}

fn a_row_without_the_column_is_refused() -> Result<[u32; 4], OperatingError> {
    found(&[], TreeForm::Elements, (0, 0), 0)
}

fn it_applies_to_a_finding_by_node_alone() -> [u32; 4] {
    let by_node = Some(Finding::Node(ColumnIndex(0), ColumnIndex(0)));
    let by_span = Some(Finding::Span(ColumnIndex(0)));
    four(vec![
        u32::from(EntityVectorItem.applies(by_node) == Fitting::Fits),
        u32::from(EntityVectorItem.applies(by_span) == Fitting::Fits),
        u32::from(EntityVectorItem.applies(None) == Fitting::Fits),
    ])
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_node_names_its_element"),
        word: Word::Succeeds,
        input: || a_node_names_its_element,
        expect: Expect::Value([2, NONE, NONE, NONE]),
    },
    Case {
        name: CaseName("a_node_names_the_edge_that_leads_to_it"),
        word: Word::Succeeds,
        input: || a_node_names_the_edge_that_leads_to_it,
        expect: Expect::Value([2, NONE, NONE, NONE]),
    },
    Case {
        name: CaseName("the_root_is_led_to_by_no_edge"),
        word: Word::Succeeds,
        input: || the_root_is_led_to_by_no_edge,
        expect: Expect::Value([NONE, NONE, NONE, NONE]),
    },
    Case {
        name: CaseName("a_parent_names_the_children_of_its_edges"),
        word: Word::Succeeds,
        input: || a_parent_names_the_children_of_its_edges,
        expect: Expect::Value([1, 2, NONE, NONE]),
    },
    Case {
        name: CaseName("a_node_of_another_file_names_nothing"),
        word: Word::Succeeds,
        input: || a_node_of_another_file_names_nothing,
        expect: Expect::Value([NONE, NONE, NONE, NONE]),
    },
    Case {
        name: CaseName("it_applies_to_a_finding_by_node_alone"),
        word: Word::Succeeds,
        input: || || Ok(it_applies_to_a_finding_by_node_alone()),
        expect: Expect::Value([1, 0, 0, NONE]),
    },
    Case {
        name: CaseName("a_row_without_the_column_is_refused"),
        word: Word::Refuses,
        input: || a_row_without_the_column_is_refused,
        expect: Expect::Error(OperatingError::Table(TableModelError::NoSuchColumn {
            index: 0,
        })),
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
