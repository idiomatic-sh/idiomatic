//! Physical rows of the selected, zero-width and native representations, through the registry.
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_cells::types::{TreeForm, TreeRows};
use crate::domain::table_model::service::{borrowed, kind, registered};
use crate::domain::table_model::collections::representations::types::*;
use crate::domain::table_model::service_test::{number, column, selection, values, check_rows};
use crate::domain::table_model::types::{PositionedRows, Rows, SelectedRows, StoredSelection};
use ryo_language::domain::builders::service::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::completion::service::{completion, name_kind};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::service::{node_id, to_bits};
use ryo_relations::domain::relations::types::{CellKind, ColumnsView, FileId, KindBits, Offset, Position, Span, Value as Cell};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};
use std::sync::Arc;

/// A scenario: what a row asks of the part.
type Input = fn() -> Result<Value, Failure>;
/// What a scenario answers: a yes as one.
type Value = usize;
/// Why a scenario is refused.
type Failure = OperatingError;

fn selected_projection_order_duplicates_and_source_borrow_do_not_alias() -> Result<Value, Failure> {
    let first = [column(&[2, 3]), column(&[8, 9])];
    let second = [column(&[12, 13]), column(&[18, 19])];
    let positions = selection(
        vec![1, 0, 1],
        vec![ColumnIndex(1), ColumnIndex(0), ColumnIndex(1)],
        vec![CellKind::Number; 3],
    );
    for (columns, offset) in [(&first[..], 0), (&second[..], 10), (&first[..], 0)] {
        let input = Rows::Selected(SelectedRows {
            source: ColumnsView {
                kinds: &[CellKind::Number; 2],
                columns,
            },
            selection: Arc::clone(&positions),
        });
        let expected = [[9, 3, 9], [8, 2, 8], [9, 3, 9]]
            .map(|row| row.map(|value| number(value + offset)).to_vec());
        check_rows(&input, &expected);
    }
    Ok(1)
}

fn selected_metadata_and_missing_positions_never_fabricate_rows() -> Result<Value, Failure> {
    let columns = [column(&[2, 3]), column(&[8])];
    let source = ColumnsView {
        kinds: &[CellKind::Number; 2],
        columns: &columns,
    };
    let item = registered(RepresentationKind::Selected);
    for (columns, kinds) in [
        (vec![ColumnIndex(9)], vec![CellKind::Number]),
        (vec![ColumnIndex(0)], vec![CellKind::Symbol]),
        (vec![ColumnIndex(0)], vec![]),
    ] {
        let selected = SelectedRows {
            source,
            selection: selection(vec![0], columns, kinds),
        };
        let input = Rows::Selected(selected);
        let contract = item
            .prepare(borrowed(&input))
            .expect("metadata refusal is deferred");
        assert!(matches!(
            item.inspect(&contract),
            Err(OperatingError::ShapeMismatch)
        ));
    }
    let selected = Rows::Selected(SelectedRows {
        source,
        selection: selection(vec![1, 9, 0], vec![ColumnIndex(0)], vec![CellKind::Number]),
    });
    let raw = borrowed(&selected);
    let contract = item.prepare(raw).unwrap();
    assert_eq!(
        item.inspect(&contract).unwrap(),
        PhysicalShape::ReadablePhysicalFallback
    );
    let mut decode = [number(99)];
    assert_eq!(
        values(item.row(raw, RowIndex(0), &mut decode).unwrap()),
        &[number(3)]
    );
    assert!(matches!(
        item.row(raw, RowIndex(1), &mut decode).unwrap(),
        RowRead::MissingPhysical
    ));
    assert_eq!(
        values(item.row(raw, RowIndex(2), &mut decode).unwrap()),
        &[number(2)]
    );
    Ok(1)
}

fn zero_width_population_preserves_each_logical_row_without_inventing_stored_rows() -> Result<Value, Failure> {
    let table = Table {
        kinds: vec![],
        cells: vec![],
        count: RowCount(3),
    };
    check_rows(&Rows::Computed(&table), &[vec![], vec![], vec![]]);
    let empty = ColumnsView {
        kinds: &[],
        columns: &[],
    };
    check_rows(&Rows::Stored(empty), &[]);
    let selected = Rows::Selected(SelectedRows {
        source: empty,
        selection: selection(vec![9, 0, 9], vec![], vec![]),
    });
    check_rows(&selected, &[vec![], vec![], vec![]]);
    Ok(1)
}

fn native_elements_and_edges_preserve_utf8_spans_and_do_not_publish_end_sentinels() -> Result<Value, Failure> {
    let bytes = "é".as_bytes();
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(bytes));
    enter_node(&mut builder, KindBits(0), Offset(0));
    token(&mut builder, KindBits(1), Offset(0), Offset(2));
    leave_node(&mut builder, Offset(2));
    let mut named = completion(&mut elements, &mut symbols, ByteSlice(bytes), FileId(7));
    name_kind(&mut named, KindBits(0), NameText("module"));
    name_kind(&mut named, KindBits(1), NameText("name"));
    let item = registered(RepresentationKind::Tree);
    for form in [TreeForm::Elements, TreeForm::Edges] {
        let input = Rows::Tree(TreeRows {
            elements: &elements,
            file: FileId(7),
            form,
        });
        let raw = borrowed(&input);
        let contract = item.prepare(raw).unwrap();
        assert_eq!(item.inspect(&contract).unwrap(), PhysicalShape::Complete);
        assert_eq!(
            contract.rows,
            RowCount(if form == TreeForm::Elements { 2 } else { 1 })
        );
        let mut decode = [number(0); 5];
        let row = values(item.row(raw, RowIndex(0), &mut decode).unwrap());
        if form == TreeForm::Elements {
            assert_eq!(
                row[4],
                Cell::Span(Span {
                    start: Offset(0),
                    end: Offset(2)
                })
            );
        } else {
            assert_eq!(row[0], Cell::Node(node_id(FileId(7), Position(0))));
            assert_eq!(row[1], Cell::Node(node_id(FileId(7), Position(1))));
        }
        assert!(matches!(
            item.row(raw, RowIndex(contract.rows.0), &mut decode).unwrap(),
            RowRead::MissingPhysical
        ));
    }
    Ok(1)
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("selected_projection_order_duplicates_and_source_borrow_do_not_alias"),
        word: Word::Succeeds,
        input: || selected_projection_order_duplicates_and_source_borrow_do_not_alias,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("selected_metadata_and_missing_positions_never_fabricate_rows"),
        word: Word::Succeeds,
        input: || selected_metadata_and_missing_positions_never_fabricate_rows,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("zero_width_population_preserves_each_logical_row_without_inventing_stored_rows"),
        word: Word::Succeeds,
        input: || zero_width_population_preserves_each_logical_row_without_inventing_stored_rows,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("native_elements_and_edges_preserve_utf8_spans_and_do_not_publish_end_sentinels"),
        word: Word::Succeeds,
        input: || native_elements_and_edges_preserve_utf8_spans_and_do_not_publish_end_sentinels,
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
