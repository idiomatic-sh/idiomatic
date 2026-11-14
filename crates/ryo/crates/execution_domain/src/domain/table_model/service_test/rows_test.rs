//! The `table_model` service's cases for reading rows of stored, tree and computed tables, on values alone.

use crate::domain::table_model::service::{count_of, kinds_of, read_row};
use crate::domain::table_model::service_test::pairs;
use crate::domain::table_model::types::Rows;
use ryo_language::domain::builders::service::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::completion::service::{completion, name_kind};
use ryo_language::domain::elements::service::kind_names;
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_operators::domain::tree_cells::types::{TreeForm, TreeRows};
use ryo_relations::domain::relations::constants::NO_SYMBOL;
use ryo_relations::domain::relations::service::node_id;
use ryo_relations::domain::relations::types::{
    CellKind, ColumnsView, FactColumn, FileId, Flag, Integer, KindBits, Offset, Position, Span,
    Value as Cell,
};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::rows::errors::RowsError;
use ryo_values::domain::rows::service::reset;
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};

/// A scenario: what a row asks of the part.
type Input = fn() -> Result<Value, RowsError>;
/// What a scenario answers: a yes as one.
type Value = usize;
/// Why a scenario is refused.
type Failure = RowsError;

/// A stored relation reads row by row, each cell as its kind says, with the same room for every row.
fn reads_a_stored_row_checked() -> Value {
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
fn reads_a_stored_row() -> Result<Value, Failure> {
    Ok(reads_a_stored_row_checked())
}

/// The elements of a tree are rows of `syntax-tree` and its edges are rows of `syntax-child`, read from the table.
fn reads_the_rows_of_a_tree_checked() -> Value {
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
fn reads_the_rows_of_a_tree() -> Result<Value, Failure> {
    Ok(reads_the_rows_of_a_tree_checked())
}

/// A table computed already is looked up where it is, and the rows of a store are made into a table.
fn a_computed_table_is_not_copied_checked() -> Value {
    const KINDS: &[CellKind] = &[CellKind::Number];
    let computed = pairs(&[[1, 2]]);
    let columns = [FactColumn(vec![7, 8, 9])];
    let stored = Rows::Stored(ColumnsView {
        kinds: KINDS,
        columns: &columns,
    });
    let mut scratch = Vec::new();
    let computed_rows = Rows::Computed(&computed);
    let computed_view =
        crate::domain::table_model::types::RepresentationInput::from(&computed_rows);
    let read = computed_view
        .read(RowIndex(0), &mut scratch)
        .expect("computed row");
    let same = std::ptr::eq(read.as_ptr(), computed.cells.as_ptr()) && scratch.is_empty();
    let stored_view = crate::domain::table_model::types::RepresentationInput::from(&stored);
    let copied = (0..stored_view.count().0)
        .map(|at| {
            stored_view
                .read(RowIndex(at), &mut scratch)
                .and_then(|row| row.first().copied())
        })
        .collect::<Option<Vec<_>>>();
    usize::from(
        same && copied
            == Some(vec![
                Cell::Number(Integer(7)),
                Cell::Number(Integer(8)),
                Cell::Number(Integer(9)),
            ]),
    )
}
fn a_computed_table_is_not_copied() -> Result<Value, Failure> {
    Ok(a_computed_table_is_not_copied_checked())
}

/// A table emptied for the next file keeps the room a big file grew it to: the worker owns it to the end of the run.
fn a_table_emptied_keeps_its_room_checked() -> Value {
    let mut table = Table {
        kinds: vec![CellKind::Number],
        ..Table::default()
    };
    table.cells.reserve(1 << 20);
    let before = table.cells.capacity();
    reset(&mut table, &[CellKind::Number]);
    usize::from(table.cells.capacity() == before && before >= 1 << 20)
}
fn a_table_emptied_keeps_its_room() -> Result<Value, Failure> {
    Ok(a_table_emptied_keeps_its_room_checked())
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_stored_row_is_read_in_place_as_its_kinds"),
        word: Word::Succeeds,
        input: || reads_a_stored_row,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_rows_of_a_tree_are_read_from_its_table"),
        word: Word::Succeeds,
        input: || reads_the_rows_of_a_tree,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_computed_table_is_looked_up_where_it_is"),
        word: Word::Succeeds,
        input: || a_computed_table_is_not_copied,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_table_emptied_keeps_its_room"),
        word: Word::Succeeds,
        input: || a_table_emptied_keeps_its_room,
        expect: Expect::Value(1),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
