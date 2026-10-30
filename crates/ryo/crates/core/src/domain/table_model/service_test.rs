//! The `table_model` service's cases, on values alone.

use ryo_language::domain::completion::service::{completion, name_kind};
use ryo_language::domain::elements::service::kind_names;
use ryo_language::domain::elements::service::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::relations::constants::NO_SYMBOL;
use ryo_language::domain::relations::service::node_id;
use ryo_language::domain::relations::types::Flag;
use ryo_language::domain::relations::types::Integer;
use ryo_language::domain::relations::types::{
    CellKind, FileId, KindBits, Offset, Position, Span, Value as Cell,
};
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::symbols::types::Symbols;
use ryo_language::domain::terms::types::NameText;

use super::errors::TableModelError;
use super::service::{
    cell, count_of, index, kinds_of, matches_empty, matching, push_row, read_row, reset, table_in,
};
use super::types::{Emptiness, KeyIndex, On, RowCount, Rows, Table};
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::domain::table_cells::types::{RowIndex, TreeForm, TreeRows};
use ryo_language::domain::relations::types::{ColumnsView, FactColumn};
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// A scenario: what a row asks of the part.
type Input = fn() -> Result<Value, TableModelError>;
/// What a scenario answers: a yes as one, or a count.
type Value = usize;
/// Why a scenario is refused.
type Failure = TableModelError;

fn finds_the_second_cell() -> Result<Value, TableModelError> {
    let row = [Cell::Number(Integer(1)), Cell::Number(Integer(2))];
    Ok(usize::from(
        cell(&row, ColumnIndex(1))? == Cell::Number(Integer(2)),
    ))
}

fn misses_a_cell() -> Result<Value, TableModelError> {
    cell(&[Cell::Number(Integer(1))], ColumnIndex(4)).map(|_| 0)
}

/// A table of two numbers a row.
fn pairs(rows: &[[i64; 2]]) -> Table {
    let mut table = Table {
        kinds: vec![CellKind::Number; 2],
        ..Table::default()
    };
    for [left, right] in rows {
        push_row(
            &mut table,
            &[Cell::Number(Integer(*left)), Cell::Number(Integer(*right))],
        );
    }
    table
}

/// Looking up a key finds the rows that have it, in the order they came, whatever the order of the table.
fn finds_the_rows_of_a_key() -> Result<Value, TableModelError> {
    let table = pairs(&[[2, 1], [1, 2], [2, 3], [3, 4]]);
    let (on, mut order) = ([ColumnIndex(0)], KeyIndex::default());
    index(&table, On::Columns(&on), &mut order)?;
    let found: Vec<RowIndex> = matching(
        &table,
        On::Columns(&on),
        &order,
        &[Cell::Number(Integer(2))],
    )
    .copied()
    .collect();
    let none = matching(
        &table,
        On::Columns(&on),
        &order,
        &[Cell::Number(Integer(9))],
    );
    Ok(usize::from(
        found == [RowIndex(0), RowIndex(2)] && matches_empty(&none) == Emptiness::Empty,
    ))
}

/// Among many keys, a key finds exactly the rows that have it, still in the order they came.
fn finds_a_key_among_many() -> Result<Value, TableModelError> {
    let rows: Vec<[i64; 2]> = (0..700).map(|each| [each % 7, each]).collect();
    let table = pairs(&rows);
    let (on, mut order) = ([ColumnIndex(0)], KeyIndex::default());
    index(&table, On::Columns(&on), &mut order)?;
    let found: Vec<RowIndex> = matching(
        &table,
        On::Columns(&on),
        &order,
        &[Cell::Number(Integer(3))],
    )
    .copied()
    .collect();
    let expected: Vec<RowIndex> = (0..100).map(|each| RowIndex(3 + 7 * each)).collect();
    Ok(usize::from(
        found == expected
            && matches_empty(&matching(
                &table,
                On::Columns(&on),
                &order,
                &[Cell::Number(Integer(7))],
            )) == Emptiness::Empty,
    ))
}

/// A stored relation reads row by row, each cell as its kind says, with the same room for every row.
fn reads_a_stored_row() -> Value {
    const KINDS: &[CellKind] = &[CellKind::Number, CellKind::Flag];
    let columns = [FactColumn(vec![7, 8]), FactColumn(vec![0, 1])];
    let rows = Rows::Stored(ColumnsView {
        kinds: KINDS,
        columns: &columns,
    });
    let mut scratch = Vec::new();
    let second = read_row(&rows, RowIndex(1), &mut scratch).map(<[Cell]>::to_vec);
    let missing = read_row(&rows, RowIndex(2), &mut scratch).is_none();
    usize::from(
        second == Some(vec![Cell::Number(Integer(8)), Cell::Flag(Flag::Raised)])
            && missing
            && count_of(&rows) == RowCount(2),
    )
}

/// The elements of a tree are rows of `syntax-tree` and its edges are rows of `syntax-child`, read from the table.
fn reads_the_rows_of_a_tree() -> Value {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"abcd"));
    enter_node(&mut builder, KindBits(1), Offset(0));
    token(&mut builder, KindBits(2), Offset(0), Offset(2));
    leave_node(&mut builder, Offset(4));
    name_kind(
        &mut completion(&mut elements, &mut symbols, ByteSlice(b"abcd"), FileId(0)),
        KindBits(1),
        NameText("file"),
    );
    name_kind(
        &mut completion(&mut elements, &mut symbols, ByteSlice(b"abcd"), FileId(0)),
        KindBits(2),
        NameText("word"),
    );
    let (names, text) = (
        kind_names(&elements)[1],
        ryo_language::domain::elements::service::text_at(&elements, Position(1)),
    );
    let nodes = Rows::Tree(TreeRows {
        elements: &elements,
        file: FileId(3),
        form: TreeForm::Elements,
    });
    let edges = Rows::Tree(TreeRows {
        elements: &elements,
        file: FileId(3),
        form: TreeForm::Edges,
    });
    let mut scratch = Vec::new();
    let token_row = read_row(&nodes, RowIndex(1), &mut scratch).map(<[Cell]>::to_vec);
    let edge_row = read_row(&edges, RowIndex(0), &mut scratch).map(<[Cell]>::to_vec);
    let id = |at: u32| Cell::Node(node_id(FileId(3), Position(at)));
    let expected_token = vec![
        id(1),
        Cell::Symbol(kind_names(&elements)[2]),
        Cell::Symbol(text),
        Cell::File(FileId(3)),
        Cell::Span(Span {
            start: Offset(0),
            end: Offset(2),
        }),
    ];
    usize::from(
        count_of(&nodes) == RowCount(2)
            && count_of(&edges) == RowCount(1)
            && token_row == Some(expected_token)
            && edge_row
                == Some(vec![
                    id(0),
                    id(1),
                    Cell::Number(Integer(0)),
                    Cell::Symbol(NO_SYMBOL),
                    Cell::File(FileId(3)),
                ])
            && edge_row.as_ref().map(Vec::len) == Some(kinds_of(&edges).len())
            && token_row.as_ref().map(Vec::len) == Some(kinds_of(&nodes).len())
            && names != kind_names(&elements)[2]
            && read_row(&edges, RowIndex(1), &mut scratch).is_none(),
    )
}

/// A table computed already is looked up where it is, and the rows of a store are made into a table.
fn a_computed_table_is_not_copied() -> Value {
    const KINDS: &[CellKind] = &[CellKind::Number];
    let computed = pairs(&[[1, 2]]);
    let columns = [FactColumn(vec![7, 8, 9])];
    let stored = Rows::Stored(ColumnsView {
        kinds: KINDS,
        columns: &columns,
    });
    let (mut room, mut scratch) = (Table::default(), Vec::new());
    let same = std::ptr::eq(
        table_in(&Rows::Computed(&computed), &mut room, &mut scratch),
        &raw const computed,
    ) && room.count.0 == 0;
    let made = table_in(&stored, &mut room, &mut scratch).count.0 == 3;
    usize::from(same && made)
}

/// A table emptied for the next file keeps the room a big file grew it to: the worker owns it to the end of the run.
fn a_table_emptied_keeps_its_room() -> Value {
    let mut table = Table {
        kinds: vec![CellKind::Number],
        ..Table::default()
    };
    table.cells.reserve(1 << 20);
    let before = table.cells.capacity();
    reset(&mut table, &[CellKind::Number]);
    usize::from(table.cells.capacity() == before && before >= 1 << 20)
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_cell_is_found_by_position"),
        word: Word::Succeeds,
        input: || finds_the_second_cell,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_key_finds_its_rows_in_the_order_they_came"),
        word: Word::Succeeds,
        input: || finds_the_rows_of_a_key,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_key_among_many_finds_exactly_its_rows"),
        word: Word::Succeeds,
        input: || finds_a_key_among_many,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_stored_row_is_read_in_place_as_its_kinds"),
        word: Word::Succeeds,
        input: || || Ok(reads_a_stored_row()),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_rows_of_a_tree_are_read_from_its_table"),
        word: Word::Succeeds,
        input: || || Ok(reads_the_rows_of_a_tree()),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_computed_table_is_looked_up_where_it_is"),
        word: Word::Succeeds,
        input: || || Ok(a_computed_table_is_not_copied()),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_table_emptied_keeps_its_room"),
        word: Word::Succeeds,
        input: || || Ok(a_table_emptied_keeps_its_room()),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_position_the_row_lacks_is_refused"),
        word: Word::Refuses,
        input: || misses_a_cell,
        expect: Expect::Refusal(|error| {
            selects(matches!(error, TableModelError::NoSuchColumn { .. }))
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
