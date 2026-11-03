//! The `span_index` item's cases: the elements that cover a stretch.

use ryo_language::domain::elements::service::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::elements::types::Elements;
use ryo_relations::domain::relations::service::node_id;
use ryo_relations::domain::relations::types::{
    CellKind, FileId, KindBits, Offset, Position, Relation, Span, Value as Cell,
};
use ryo_language::domain::symbols::types::ByteSlice;

use ryo_language::domain::symbols::types::Symbols;

use crate::application::file_dag::testing::TestOperator as _;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::operator::collections::operators::join::item::JoinItem;
use crate::domain::table_cells::types::{TreeForm, TreeRows};
use crate::domain::table_model::errors::TableModelError;
use crate::domain::table_model::service::push_row;
use crate::domain::table_model::types::{Rows, Table};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use crate::domain::tree_join::collections::strategies::interface::StrategiesItem as _;
use crate::domain::tree_join::collections::strategies::span_index::item::SpanIndexItem;
use crate::domain::tree_join::collections::strategies::types::{Finding, Fitting};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as PlanInput, JoinColumn, Side};
use ryo_values::domain::plans::types::Operation;

/// What a row asks: the stretch on the left, and how it is held.
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

fn found(row: &[Cell], finding: Finding, file: u32) -> Result<[u32; 4], OperatingError> {
    let elements = tree();
    let rows = TreeRows {
        elements: &elements,
        file: FileId(file),
        form: TreeForm::Elements,
    };
    let mut found = Vec::new();
    SpanIndexItem::find(&rows, row, finding, &mut found)?;
    Ok(four(found.into_iter().map(|each| each.0).collect()))
}

fn stretch(start: u32, end: u32) -> Cell {
    Cell::Span(Span {
        start: Offset(start),
        end: Offset(end),
    })
}

fn reordered_starts_keep_exact_span_matches() -> Result<[u32; 4], OperatingError> {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"abcd"));
    enter_node(&mut builder, KindBits(1), Offset(0));
    enter_node(&mut builder, KindBits(2), Offset(2));
    leave_node(&mut builder, Offset(4));
    ryo_language::domain::elements::service::enter_implied_node(&mut builder, KindBits(3));
    leave_node(&mut builder, Offset(0));
    token(&mut builder, KindBits(4), Offset(0), Offset(2));
    leave_node(&mut builder, Offset(4));
    let rows = TreeRows {
        elements: &elements,
        file: FileId(0),
        form: TreeForm::Elements,
    };
    let mut found = Vec::new();
    SpanIndexItem::find(
        &rows,
        &[stretch(0, 2)],
        Finding::Span(ColumnIndex(0)),
        &mut found,
    )?;
    Ok(four(found.into_iter().map(|each| each.0).collect()))
}

fn a_stretch_names_the_elements_that_cover_it_exactly() -> Result<[u32; 4], OperatingError> {
    // The node at 2 and its token both cover 1..2; the root covers 0..2 and the first token 0..1.
    found(&[stretch(1, 2)], Finding::Span(ColumnIndex(0)), 0)
}

fn a_stretch_nothing_covers_exactly_names_nothing() -> Result<[u32; 4], OperatingError> {
    found(&[stretch(0, 5)], Finding::Span(ColumnIndex(0)), 0)
}

fn a_stretch_of_the_file_is_found_by_the_file_and_the_stretch() -> Result<[u32; 4], OperatingError>
{
    found(
        &[Cell::File(FileId(0)), stretch(0, 1)],
        Finding::Stretch(ColumnIndex(0), ColumnIndex(1)),
        0,
    )
}

fn a_stretch_of_another_file_names_nothing() -> Result<[u32; 4], OperatingError> {
    found(
        &[Cell::File(FileId(1)), stretch(0, 1)],
        Finding::Stretch(ColumnIndex(0), ColumnIndex(1)),
        0,
    )
}

fn a_row_without_the_column_is_refused() -> Result<[u32; 4], OperatingError> {
    found(&[], Finding::Span(ColumnIndex(0)), 0)
}

/// Joins rows of a file and a stretch with the elements of the file that cover exactly that stretch, answering their ids.
fn joins_on_the_stretch(stretch_first: bool) -> Result<Table, OperatingError> {
    let elements = tree();
    let mut symbols = Symbols::default();
    let file = Cell::File(FileId(0));
    let shift = usize::from(!stretch_first);
    let mut kinds = [CellKind::Span, CellKind::File];
    kinds.rotate_left(shift);
    let mut left = Table {
        kinds: kinds.to_vec(),
        ..Table::default()
    };
    for (start, end) in [(1, 2), (0, 2), (7, 9)] {
        let mut row = [stretch(start, end), file];
        row.rotate_left(shift);
        push_row(&mut left, &row);
    }
    let columns = |places: &[u32]| places.iter().map(|place| ColumnIndex(*place)).collect();
    let right_on: &[u32] = if stretch_first { &[4, 3] } else { &[3, 4] };
    let operation = Operation::Join {
        left: PlanInput::Relation(Relation::Function),
        right: PlanInput::Relation(Relation::Function),
        left_on: columns(&[0, 1]),
        right_on: columns(right_on),
        output: vec![JoinColumn {
            side: Side::Right,
            column: ColumnIndex(0),
        }],
        filter: Vec::new(),
        view: Vec::new(),

        reach: None,
    };
    let rows = TreeRows {
        elements: &elements,
        file: FileId(0),
        form: TreeForm::Elements,
    };
    let mut out = Table::default();
    let mut context = OperatorContext {
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    JoinItem.compute(
        &operation,
        &[Rows::Computed(&left), Rows::Tree(rows)],
        &mut context,
        &mut out,
    )?;
    Ok(out)
}

fn either_order_finds_the_covering_elements() -> Result<[u32; 4], OperatingError> {
    let node = |position: u32| Cell::Node(node_id(FileId(0), Position(position)));
    let expected = [node(2), node(3), node(0)];
    let (file_first, stretch_first) = (joins_on_the_stretch(false)?, joins_on_the_stretch(true)?);
    Ok(four(vec![u32::from(
        file_first.cells == expected && stretch_first.cells == expected,
    )]))
}

fn it_applies_to_a_finding_by_stretch_alone() -> [u32; 4] {
    let by_node = Some(Finding::Node(ColumnIndex(0), ColumnIndex(0)));
    let by_span = Some(Finding::Span(ColumnIndex(0)));
    let by_stretch = Some(Finding::Stretch(ColumnIndex(0), ColumnIndex(1)));
    four(vec![
        u32::from(SpanIndexItem.applies(by_node) == Fitting::Fits),
        u32::from(SpanIndexItem.applies(by_span) == Fitting::Fits),
        u32::from(SpanIndexItem.applies(by_stretch) == Fitting::Fits),
        u32::from(SpanIndexItem.applies(None) == Fitting::Fits),
    ])
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("reordered_starts_keep_exact_span_matches"),
        word: Word::Succeeds,
        input: || reordered_starts_keep_exact_span_matches,
        expect: Expect::Value([3, NONE, NONE, NONE]),
    },
    Case {
        name: CaseName("a_stretch_names_the_elements_that_cover_it_exactly"),
        word: Word::Succeeds,
        input: || a_stretch_names_the_elements_that_cover_it_exactly,
        expect: Expect::Value([2, 3, NONE, NONE]),
    },
    Case {
        name: CaseName("a_stretch_nothing_covers_exactly_names_nothing"),
        word: Word::Succeeds,
        input: || a_stretch_nothing_covers_exactly_names_nothing,
        expect: Expect::Value([NONE, NONE, NONE, NONE]),
    },
    Case {
        name: CaseName("a_stretch_of_the_file_is_found_by_the_file_and_the_stretch"),
        word: Word::Succeeds,
        input: || a_stretch_of_the_file_is_found_by_the_file_and_the_stretch,
        expect: Expect::Value([1, NONE, NONE, NONE]),
    },
    Case {
        name: CaseName("a_stretch_of_another_file_names_nothing"),
        word: Word::Succeeds,
        input: || a_stretch_of_another_file_names_nothing,
        expect: Expect::Value([NONE, NONE, NONE, NONE]),
    },
    Case {
        name: CaseName("the_elements_covering_a_stretch_are_found_by_it_in_either_order"),
        word: Word::Succeeds,
        input: || either_order_finds_the_covering_elements,
        expect: Expect::Value([1, NONE, NONE, NONE]),
    },
    Case {
        name: CaseName("it_applies_to_a_finding_by_stretch_alone"),
        word: Word::Succeeds,
        input: || || Ok(it_applies_to_a_finding_by_stretch_alone()),
        expect: Expect::Value([0, 1, 1, 0]),
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
