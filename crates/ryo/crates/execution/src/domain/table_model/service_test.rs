//! The `table_model` service's cases, on values alone.

use ryo_language::domain::completion::service::{completion, name_kind};
use ryo_language::domain::elements::service::kind_names;
use ryo_language::domain::elements::service::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::constants::NO_SYMBOL;
use ryo_relations::domain::relations::service::node_id;
use ryo_relations::domain::relations::types::Flag;
use ryo_relations::domain::relations::types::Integer;
use ryo_relations::domain::relations::types::{
    CellKind, FileId, KindBits, Offset, Position, Span, Value as Cell,
};
use ryo_relations::domain::terms::types::NameText;

use ryo_values::domain::rows::errors::TableModelError;
use {ryo_values::domain::rows::service::cell, super::service::count_of, super::service::kinds_of, super::service::matching, super::service::push_row, super::service::read_row, super::service::reset};
use super::types::Rows;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::domain::positions::service_test::prepared_index;
use crate::domain::table_cells::types::{TreeForm, TreeRows};
use ryo_derived::domain::contract::types::On;
use ryo_relations::domain::relations::types::{ColumnsView, FactColumn};
use ryo_values::domain::plan_parts::types::ColumnIndex;

#[test]
fn compact_materialization_decodes_projected_cells_without_full_row_reads() {
    use super::service::{materialize, take_read_row_counts};
    use super::types::{SelectedRows, StoredSelection};
    use ryo_relations::domain::relations::{service::to_bits, types::Relation};
    use std::sync::Arc;

    let physical = [
        FactColumn(
            [1, 2, 3]
                .map(|value| to_bits(Cell::Number(Integer(value))).0)
                .to_vec(),
        ),
        FactColumn(
            [8, 9]
                .map(|value| to_bits(Cell::Number(Integer(value))).0)
                .to_vec(),
        ),
    ];
    let selected = Rows::Selected(SelectedRows {
        source: ColumnsView {
            kinds: &[CellKind::Number, CellKind::Number],
            columns: &physical,
        },
        selection: Arc::new(StoredSelection {
            relation: Relation::Function,
            rows: Arc::new(vec![1, 0, 1, 2]),
            columns: vec![ColumnIndex(1), ColumnIndex(0), ColumnIndex(1)],
            kinds: vec![CellKind::Number; 3],
            identity: Arc::new(Default::default()),
        }),
    });
    // The fourth physical row is malformed. Its first projected cell is absent,
    // so both paths omit it while retaining duplicate logical positions.
    let _previous = take_read_row_counts();
    let mut actual = Table::default();
    materialize(&selected, &mut actual, &mut Vec::new());
    assert_eq!(take_read_row_counts(), (0, 0));
    assert_eq!(actual.count.0, 3);
    assert_eq!(
        actual.cells,
        vec![
            Cell::Number(Integer(9)),
            Cell::Number(Integer(2)),
            Cell::Number(Integer(9)),
            Cell::Number(Integer(8)),
            Cell::Number(Integer(1)),
            Cell::Number(Integer(8)),
            Cell::Number(Integer(9)),
            Cell::Number(Integer(2)),
            Cell::Number(Integer(9)),
        ]
    );
    let mut reference = Table {
        kinds: actual.kinds.clone(),
        ..Table::default()
    };
    let mut scratch = Vec::new();
    for index in 0..count_of(&selected).0 {
        if let Some(row) = read_row(&selected, RowIndex(index), &mut scratch) {
            push_row(&mut reference, row);
        }
    }
    assert_eq!(actual, reference);
}

#[test]
fn compact_materialization_preserves_ragged_and_zero_width_legacy_rows() {
    use super::service::materialize;
    use super::types::{SelectedRows, StoredSelection};
    use ryo_relations::domain::relations::{service::to_bits, types::Relation};
    use std::sync::Arc;

    let physical = [
        FactColumn(vec![
            to_bits(Cell::Number(Integer(1))).0,
            to_bits(Cell::Number(Integer(2))).0,
        ]),
        FactColumn(vec![to_bits(Cell::Number(Integer(9))).0]),
    ];
    let source = ColumnsView {
        kinds: &[CellKind::Number, CellKind::Number],
        columns: &physical,
    };
    let selection = |rows, columns, kinds| {
        Rows::Selected(SelectedRows {
            source,
            selection: Arc::new(StoredSelection {
                relation: Relation::Function,
                rows: Arc::new(rows),
                columns,
                kinds,
                identity: Arc::new(Default::default()),
            }),
        })
    };

    // An unused physical column may be ragged; only selected cells are read.
    let mut projected = Table::default();
    materialize(
        &selection(vec![1, 0], vec![ColumnIndex(0)], vec![CellKind::Number]),
        &mut projected,
        &mut Vec::new(),
    );
    assert_eq!(projected.count.0, 2);
    assert_eq!(
        projected.cells,
        vec![Cell::Number(Integer(2)), Cell::Number(Integer(1))]
    );

    // An invalid selected column skips its row without publishing partial cells.
    let mut invalid_column = Table::default();
    materialize(
        &selection(vec![0], vec![ColumnIndex(5)], vec![CellKind::Number]),
        &mut invalid_column,
        &mut Vec::new(),
    );
    assert_eq!(invalid_column.count.0, 0);
    assert!(invalid_column.cells.is_empty());

    // Legacy read_row did not revalidate a malformed declared kind. Retain the
    // same table shape and decoded source value until the admission layer rejects it.
    let mut invalid_kind = Table::default();
    materialize(
        &selection(vec![0], vec![ColumnIndex(0)], vec![CellKind::Symbol]),
        &mut invalid_kind,
        &mut Vec::new(),
    );
    assert_eq!(invalid_kind.kinds, vec![CellKind::Symbol]);
    assert_eq!(invalid_kind.cells, vec![Cell::Number(Integer(1))]);
    assert_eq!(invalid_kind.count.0, 1);

    // Zero-width projections retain each logical row, even when its physical
    // position is beyond a provider column that the projection never reads.
    let mut zero_width = Table::default();
    materialize(
        &selection(vec![3, 0, 3], vec![], vec![]),
        &mut zero_width,
        &mut Vec::new(),
    );
    assert_eq!(zero_width.count.0, 3);
    assert!(zero_width.kinds.is_empty());
    assert!(zero_width.cells.is_empty());
}

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
    let on = [ColumnIndex(0)];
    let order = prepared_index(&Rows::Computed(&table), &on)?;
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
        found == [RowIndex(0), RowIndex(2)] && none.clone().next().is_none(),
    ))
}

/// Among many keys, a key finds exactly the rows that have it, still in the order they came.
fn finds_a_key_among_many() -> Result<Value, TableModelError> {
    let rows: Vec<[i64; 2]> = (0..700).map(|each| [each % 7, each]).collect();
    let table = pairs(&rows);
    let on = [ColumnIndex(0)];
    let order = prepared_index(&Rows::Computed(&table), &on)?;
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
            && matching(
                &table,
                On::Columns(&on),
                &order,
                &[Cell::Number(Integer(7))],
            )
            .next()
            .is_none(),
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
    let mut scratch = Vec::new();
    let computed_rows = Rows::Computed(&computed);
    let computed_view =
        crate::domain::operator::contract::views::BorrowedRows::from(&computed_rows);
    let read = computed_view
        .read(RowIndex(0), &mut scratch)
        .expect("computed row");
    let same = std::ptr::eq(read.as_ptr(), computed.cells.as_ptr()) && scratch.is_empty();
    let stored_view = crate::domain::operator::contract::views::BorrowedRows::from(&stored);
    let copied = (0..stored_view.count())
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

#[test]
fn stored_keys_preserve_order_duplicates_and_typed_flag_normalization() {
    use super::service::{matching_stored, rectangular};
    const KINDS: &[CellKind] = &[CellKind::Number, CellKind::Flag, CellKind::Number];
    let columns = [
        FactColumn(vec![2, 1, 2, 2]),
        FactColumn(vec![1, 0, 2, 0]),
        FactColumn(vec![20, 10, 21, 22]),
    ];
    let view = ColumnsView {
        kinds: KINDS,
        columns: &columns,
    };
    assert!(rectangular(view));
    let mut materialized = Table::default();
    super::service::materialize(&Rows::Stored(view), &mut materialized, &mut Vec::new());
    for key_columns in [
        vec![ColumnIndex(0)],
        vec![ColumnIndex(1), ColumnIndex(0)],
        Vec::new(),
    ] {
        let stored = prepared_index(&Rows::Stored(view), &key_columns).expect("valid stored key");
        let computed = prepared_index(&Rows::Computed(&materialized), &key_columns)
            .expect("valid computed key");
        for row in 0..materialized.count.0 {
            let row = super::service::row_in(&materialized, RowIndex(row)).expect("row");
            let key = key_columns
                .iter()
                .map(|column| super::service::cell(row, *column).expect("key column"))
                .collect::<Vec<_>>();
            assert_eq!(
                matching_stored(
                    view,
                    &key_columns,
                    &stored,
                    &key,
                    #[cfg(feature = "diagnostic-row-progress")]
                    |_, _| {},
                )
                .copied()
                .collect::<Vec<_>>(),
                matching(&materialized, On::Columns(&key_columns), &computed, &key)
                    .copied()
                    .collect::<Vec<_>>()
            );
        }
    }
    let stored = prepared_index(&Rows::Stored(view), &[ColumnIndex(1)]).expect("flag index");
    assert_eq!(
        matching_stored(
            view,
            &[ColumnIndex(1)],
            &stored,
            &[Cell::Flag(Flag::Raised)],
            #[cfg(feature = "diagnostic-row-progress")]
            |_, _| {},
        )
        .copied()
        .collect::<Vec<_>>(),
        [RowIndex(0), RowIndex(2)]
    );
    assert!(
        matching_stored(
            view,
            &[ColumnIndex(1)],
            &stored,
            &[Cell::Number(Integer(1))],
            #[cfg(feature = "diagnostic-row-progress")]
            |_, _| {},
        )
        .next()
        .is_none(),
        "equal hash words must not erase value kinds"
    );
}

#[test]
fn stored_keys_check_exact_composite_equality_after_a_real_hash_collision() {
    use super::service::matching_stored;
    const KINDS: &[CellKind] = &[CellKind::Number, CellKind::Number];
    let colliding_word = 0x517c_c1b7_2722_0a95_u64.rotate_left(5);
    let columns = [
        FactColumn(vec![0, 1, 0]),
        FactColumn(vec![0, colliding_word, 0]),
    ];
    let view = ColumnsView {
        kinds: KINDS,
        columns: &columns,
    };
    let on = [ColumnIndex(0), ColumnIndex(1)];
    let index = prepared_index(&Rows::Stored(view), &on).expect("key");
    assert_eq!(
        index.hashes()[0],
        index.hashes()[1],
        "fixture must exercise an actual full-hash collision"
    );
    assert_eq!(
        matching_stored(
            view,
            &on,
            &index,
            &[Cell::Number(Integer(0)), Cell::Number(Integer(0))],
            #[cfg(feature = "diagnostic-row-progress")]
            |_, _| {},
        )
        .copied()
        .collect::<Vec<_>>(),
        [RowIndex(0), RowIndex(2)]
    );
}

#[test]
fn stored_keys_reuse_empty_indexes_and_validate_columns_before_rows() {
    use super::service::{matching_stored, rectangular};
    const KINDS: &[CellKind] = &[CellKind::Number];
    let columns = [FactColumn(vec![7, 7])];
    let empty = [FactColumn(Vec::new())];
    let view = ColumnsView {
        kinds: KINDS,
        columns: &columns,
    };
    let vacant = ColumnsView {
        kinds: KINDS,
        columns: &empty,
    };
    let on = [ColumnIndex(0)];
    let full = prepared_index(&Rows::Stored(view), &on).expect("full");
    let order = prepared_index(&Rows::Stored(vacant), &on).expect("empty");
    assert_eq!(
        full.rows().len(),
        2,
        "new empty payload does not mutate an outstanding full index"
    );
    assert!(
        matching_stored(
            vacant,
            &on,
            &order,
            &[Cell::Number(Integer(7))],
            #[cfg(feature = "diagnostic-row-progress")]
            |_, _| {},
        )
        .next()
        .is_none()
    );
    assert_eq!(
        prepared_index(&Rows::Stored(vacant), &[ColumnIndex(1)]).err(),
        Some(TableModelError::NoSuchColumn { index: 1 })
    );
    let order = prepared_index(&Rows::Stored(view), &[]).expect("cartesian");
    assert_eq!(
        matching_stored(
            view,
            &[],
            &order,
            &[],
            #[cfg(feature = "diagnostic-row-progress")]
            |_, _| {},
        )
        .copied()
        .collect::<Vec<_>>(),
        [RowIndex(0), RowIndex(1)]
    );
    let malformed = [FactColumn(vec![1]), FactColumn(Vec::new())];
    assert!(!rectangular(ColumnsView {
        kinds: &[CellKind::Number, CellKind::Number],
        columns: &malformed
    }));
    assert!(!rectangular(ColumnsView {
        kinds: KINDS,
        columns: &malformed
    }));
}

#[test]
fn complete_source_contract_covers_native_and_stored_without_promoting_partial_or_mutable_rows() {
    use super::service::complete_for_reuse;
    use crate::domain::coverage::types::{Lacking, Partial};
    use ryo_language::domain::elements::service::{diagnose, set_syntax_acceptance};
    use ryo_language::domain::elements::types::SyntaxAcceptance;
    use ryo_relations::domain::relations::service::{kinds, to_bits};
    use ryo_relations::domain::relations::types::{Relation, ReportText};
    use ryo_values::domain::stored_facts::types::PartialCause;

    fn tree(acceptance: SyntaxAcceptance, recovered: bool) -> Elements {
        let mut elements = Elements::default();
        let mut symbols = Symbols::default();
        {
            let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"x"));
            set_syntax_acceptance(&mut builder, acceptance);
            enter_node(&mut builder, KindBits(0), Offset(0));
            token(&mut builder, KindBits(1), Offset(0), Offset(1));
            leave_node(&mut builder, Offset(1));
            if recovered {
                diagnose(
                    &mut builder,
                    ReportText("recovered syntax"),
                    Offset(0),
                    Offset(1),
                );
            }
        }
        let mut named = completion(&mut elements, &mut symbols, ByteSlice(b"x"), FileId(0));
        name_kind(&mut named, KindBits(0), NameText("module"));
        name_kind(&mut named, KindBits(1), NameText("name"));
        elements
    }

    let elements = tree(SyntaxAcceptance::Accepted, false);
    let native = Rows::Tree(TreeRows {
        elements: &elements,
        file: FileId(0),
        form: TreeForm::Elements,
    });
    // Build the same typed SyntaxTree population in columns to compare both
    // read representations under one relation contract.
    let mut tree_columns = vec![FactColumn(Vec::new()); kinds(Relation::SyntaxTree).len()];
    let mut buffer = Vec::new();
    for index in 0..count_of(&native).0 {
        let row = read_row(&native, RowIndex(index), &mut buffer).expect("native row");
        for (column, value) in tree_columns.iter_mut().zip(row) {
            column.0.push(to_bits(*value).0);
        }
    }
    let stored_tree = Rows::Stored(ColumnsView {
        kinds: kinds(Relation::SyntaxTree),
        columns: &tree_columns,
    });
    let relation = Relation::SourceLocation;
    let columns = vec![FactColumn(vec![0]); kinds(relation).len()];
    let stored = Rows::Stored(ColumnsView {
        kinds: kinds(relation),
        columns: &columns,
    });
    assert!(
        !crate::domain::visit::service::walked(&[Relation::SyntaxTree])
            .contains(&Relation::SyntaxTree),
        "native tree demand is absent from the provider walk"
    );
    assert!(complete_for_reuse(Relation::SyntaxTree, &native, &[], true));
    assert!(complete_for_reuse(
        Relation::SyntaxTree,
        &stored_tree,
        &[],
        true
    ));
    assert!(complete_for_reuse(relation, &stored, &[], true));
    assert!(!complete_for_reuse(
        Relation::SyntaxTree,
        &native,
        &[],
        false
    ));
    assert!(!complete_for_reuse(relation, &stored, &[], false));
    assert!(!complete_for_reuse(relation, &native, &[], true));
    assert!(!complete_for_reuse(
        Relation::SyntaxChild,
        &native,
        &[],
        true
    ));

    let partials = [Partial {
        relation: Relation::SyntaxTree,
        cause: PartialCause("fixture partial".into()),
        lacking: Lacking::Any,
        at: Some(FileId(0)),
    }];
    assert!(!complete_for_reuse(
        Relation::SyntaxTree,
        &native,
        &partials,
        true
    ));
    assert!(!complete_for_reuse(
        Relation::SyntaxTree,
        &stored_tree,
        &partials,
        true
    ));
    let edges = Rows::Tree(TreeRows {
        elements: &elements,
        file: FileId(0),
        form: TreeForm::Edges,
    });
    assert!(!complete_for_reuse(Relation::SyntaxTree, &edges, &[], true));
    assert!(complete_for_reuse(Relation::SyntaxChild, &edges, &[], true));
    assert!(!complete_for_reuse(
        Relation::SyntaxChild,
        &edges,
        &[],
        false
    ));
    let child_partials = [Partial {
        relation: Relation::SyntaxChild,
        cause: PartialCause("child recovery".into()),
        lacking: Lacking::Any,
        at: Some(FileId(0)),
    }];
    assert!(!complete_for_reuse(
        Relation::SyntaxChild,
        &edges,
        &child_partials,
        true
    ));
    for missing in ["parent", "ordinal"] {
        let mut absent = tree(SyntaxAcceptance::Accepted, false);
        if missing == "parent" {
            absent.parent = Default::default();
        } else {
            absent.ordinal = Default::default();
        }
        let edges = Rows::Tree(TreeRows {
            elements: &absent,
            file: FileId(0),
            form: TreeForm::Edges,
        });
        assert!(
            !complete_for_reuse(Relation::SyntaxChild, &edges, &[], true),
            "absent {missing} is not complete native child backing"
        );
    }

    for acceptance in [SyntaxAcceptance::Rejected, SyntaxAcceptance::Undetermined] {
        let elements = tree(acceptance, false);
        let rows = Rows::Tree(TreeRows {
            elements: &elements,
            file: FileId(0),
            form: TreeForm::Elements,
        });
        assert!(!complete_for_reuse(Relation::SyntaxTree, &rows, &[], true));
        let edges = Rows::Tree(TreeRows {
            elements: &elements,
            file: FileId(0),
            form: TreeForm::Edges,
        });
        assert!(!complete_for_reuse(
            Relation::SyntaxChild,
            &edges,
            &[],
            true
        ));
    }
    let recovered = tree(SyntaxAcceptance::Accepted, true);
    let rows = Rows::Tree(TreeRows {
        elements: &recovered,
        file: FileId(0),
        form: TreeForm::Elements,
    });
    assert!(!complete_for_reuse(Relation::SyntaxTree, &rows, &[], true));
    let recovered_edges = Rows::Tree(TreeRows {
        elements: &recovered,
        file: FileId(0),
        form: TreeForm::Edges,
    });
    assert!(!complete_for_reuse(
        Relation::SyntaxChild,
        &recovered_edges,
        &[],
        true
    ));

    let malformed = vec![FactColumn(vec![0]), FactColumn(Vec::new())];
    assert!(!complete_for_reuse(
        relation,
        &Rows::Stored(ColumnsView {
            kinds: kinds(relation),
            columns: &malformed
        }),
        &[],
        true,
    ));
}

use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};
