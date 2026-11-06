//! Physical representations are exercised through their production registry and public row routes.
use super::{
    service::{borrowed, kind, registered},
    types::*,
};
use crate::domain::lowering::types::{
    ElementPart, LoweredColumn, LoweredShape, RowPosition, RowWidth,
};
use crate::domain::operating::errors::OperatingError;
use crate::domain::positions::types::{Positions, Slots};
use crate::domain::table_cells::types::{TreeForm, TreeRows};
use crate::domain::table_model::types::{PositionedRows, Rows, SelectedRows, StoredSelection};
use ryo_language::domain::completion::service::{completion, name_kind};
use ryo_language::domain::elements::{
    service::{enter_node, leave_node, table_builder, token},
    types::Elements,
};
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::{
    constants::NO_SYMBOL,
    service::{node_id, to_bits},
    types::{
        CellKind, ColumnsView, FactColumn, FileId, Integer, KindBits, Offset, Position, Relation,
        Span, Value,
    },
};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::sync::Arc;

fn compact(table: &Table) -> ryo_values::domain::table_words::types::CompactTable {
    use ryo_relations::domain::relations::types::Words;
    use ryo_values::domain::stored_facts::types::KeptWords;
    use ryo_values::domain::table_words::{
        service,
        types::{DecodeMode, WordSlice},
    };
    let mut words = Words::default();
    service::encode_table(table, &[], &mut words);
    let (compact, names) = service::admit_compact(
        Arc::new(KeptWords(service::pack(WordSlice(&words.0)))),
        &mut Vec::new(),
        DecodeMode::Complete,
    )
    .unwrap();
    assert!(names.is_empty());
    assert!(service::compact_rectangular(&compact));
    compact
}

fn number(value: i64) -> Value {
    Value::Number(Integer(value))
}
fn column(values: &[i64]) -> FactColumn {
    FactColumn(
        values
            .iter()
            .map(|value| to_bits(number(*value)).0)
            .collect(),
    )
}
fn selection(
    rows: Vec<u32>,
    columns: Vec<ColumnIndex>,
    kinds: Vec<CellKind>,
) -> Arc<StoredSelection> {
    Arc::new(StoredSelection {
        relation: Relation::Function,
        rows: Arc::new(rows),
        columns,
        kinds,
        identity: Arc::new(Default::default()),
    })
}
fn values(read: RowRead<'_>) -> &[Value] {
    match read {
        RowRead::Borrowed(row) | RowRead::Decoded(row) => row,
        RowRead::MissingPhysical => panic!("declared row missing"),
    }
}
fn check_rows(input: &Rows<'_>, expected: &[Vec<Value>]) {
    let raw = borrowed(input);
    let item = registered(kind(raw));
    let contract = item.prepare(raw).expect("metadata");
    assert_eq!(item.inspect(&contract).unwrap(), PhysicalShape::Complete);
    assert_eq!(contract.rows, expected.len());
    assert_eq!(kind(item.bulk(raw).unwrap()), contract.kind);
    for (at, wanted) in expected.iter().enumerate() {
        let mut decode = vec![number(99); contract.kinds.len()];
        assert_eq!(
            values(item.row(raw, RowIndex(at), &mut decode).unwrap()),
            wanted
        );
        let mut decode = Vec::new();
        assert_eq!(
            crate::domain::table_model::service::read_row(input, RowIndex(at), &mut decode)
                .unwrap(),
            wanted
        );
        for (at_column, wanted) in wanted.iter().enumerate() {
            let at_column = ColumnIndex(u32::try_from(at_column).unwrap());
            assert_eq!(
                item.cell(raw, RowIndex(at), at_column).unwrap(),
                CellRead::Value(*wanted)
            );
            assert_eq!(
                crate::domain::table_model::service::cell_at(input, RowIndex(at), at_column),
                Some(*wanted)
            );
        }
    }
    // A zero-column Stored view has no physical row bound; callers enumerate
    // only the zero rows declared by its contract, preserving legacy behavior.
    if contract.rows != 0 || contract.kind != RepresentationKind::Stored {
        let mut decode = vec![number(0); contract.kinds.len()];
        assert!(matches!(
            item.row(raw, RowIndex(usize::MAX), &mut decode).unwrap(),
            RowRead::MissingPhysical
        ));
    }
    assert_eq!(
        item.cell(raw, RowIndex(0), ColumnIndex(u32::MAX)).unwrap(),
        CellRead::MissingPhysical
    );
}

#[test]
fn all_registered_representations_preserve_bulk_cells_rows_and_bags() {
    let columns = [column(&[2, 1])];
    let source = ColumnsView {
        kinds: &[CellKind::Number],
        columns: &columns,
    };
    let selected = SelectedRows {
        source,
        selection: selection(
            vec![1, 0, 1],
            vec![ColumnIndex(0); 2],
            vec![CellKind::Number; 2],
        ),
    };
    let table = Table {
        kinds: vec![CellKind::Number],
        cells: vec![number(8), number(7)],
        count: RowCount(2),
    };
    let compact = compact(&table);
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"ab"));
    enter_node(&mut builder, KindBits(0), Offset(0));
    token(&mut builder, KindBits(0), Offset(0), Offset(1));
    leave_node(&mut builder, Offset(2));
    let tree = TreeRows {
        elements: &elements,
        file: FileId(3),
        form: TreeForm::Edges,
    };
    let positions = Positions {
        width: RowWidth(1),
        slots: Slots(vec![1, 0, 1]),
    };
    let shape = LoweredShape {
        width: RowWidth(1),
        columns: vec![
            LoweredColumn {
                slot: RowPosition(0),
                field: ElementPart::File,
            },
            LoweredColumn {
                slot: RowPosition(0),
                field: ElementPart::Node,
            },
        ],
    };
    let positioned = PositionedRows {
        tree: TreeRows {
            form: TreeForm::Elements,
            ..tree
        },
        positions: &positions,
        shape: &shape,
        kinds: &[CellKind::File, CellKind::Node],
    };
    for variant in RepresentationKind::ALL {
        // No wildcard: adding a representation requires a real independent fixture.
        let (input, expected) = match variant {
            RepresentationKind::Stored => {
                (Rows::Stored(source), vec![vec![number(2)], vec![number(1)]])
            }
            RepresentationKind::Selected => (
                Rows::Selected(selected.clone()),
                vec![vec![number(1); 2], vec![number(2); 2], vec![number(1); 2]],
            ),
            RepresentationKind::Computed => (
                Rows::Computed(&table),
                vec![vec![number(8)], vec![number(7)]],
            ),
            RepresentationKind::Compact => (
                Rows::Compact(&compact),
                vec![vec![number(8)], vec![number(7)]],
            ),
            RepresentationKind::Positioned => (
                Rows::Positioned(positioned),
                vec![
                    vec![
                        Value::File(FileId(3)),
                        Value::Node(node_id(FileId(3), Position(1))),
                    ],
                    vec![
                        Value::File(FileId(3)),
                        Value::Node(node_id(FileId(3), Position(0))),
                    ],
                    vec![
                        Value::File(FileId(3)),
                        Value::Node(node_id(FileId(3), Position(1))),
                    ],
                ],
            ),
            RepresentationKind::Tree => (
                Rows::Tree(tree),
                vec![vec![
                    Value::Node(node_id(FileId(3), Position(0))),
                    Value::Node(node_id(FileId(3), Position(1))),
                    number(0),
                    Value::Symbol(NO_SYMBOL),
                    Value::File(FileId(3)),
                ]],
            ),
        };
        assert_eq!(kind(borrowed(&input)), *variant);
        let item = registered(*variant);
        let classified = item.prepare(borrowed(&input)).unwrap();
        for other in RepresentationKind::ALL {
            if other != variant {
                assert_eq!(
                    registered(*other).inspect(&classified),
                    Err(OperatingError::WrongOperation),
                    "a {variant:?} contract cannot be submitted to {other:?}"
                );
                let raw = borrowed(&input);
                let wrong = registered(*other);
                assert!(matches!(
                    wrong.prepare(raw),
                    Err(OperatingError::WrongOperation)
                ));
                assert!(matches!(
                    wrong.bulk(raw),
                    Err(OperatingError::WrongOperation)
                ));
                assert_eq!(
                    wrong.cell(raw, RowIndex(0), ColumnIndex(0)),
                    Err(OperatingError::WrongOperation)
                );
                let mut decode = vec![number(99); classified.kinds.len()];
                assert!(matches!(
                    wrong.row(raw, RowIndex(0), &mut decode),
                    Err(OperatingError::WrongOperation)
                ));
            }
        }
        let mut mismatched = item.prepare(borrowed(&input)).unwrap();
        mismatched.kinds = &[];
        assert_eq!(
            item.inspect(&mismatched),
            Err(OperatingError::ShapeMismatch),
            "prepared schema must agree with actual physical schema for {variant:?}"
        );
        check_rows(&input, &expected);
        let bulk = registered(*variant).bulk(borrowed(&input)).unwrap();
        match bulk {
            BulkAccess::Stored(view) => assert!(std::ptr::eq(view.columns, source.columns)),
            BulkAccess::Selected {
                source: view,
                positions,
                columns,
                ..
            } => {
                assert!(std::ptr::eq(view.columns, source.columns));
                assert!(std::ptr::eq(positions, selected.selection.rows.as_slice()));
                assert!(std::ptr::eq(columns, selected.selection.columns.as_slice()));
            }
            BulkAccess::Tree(view) => assert!(std::ptr::eq(view.elements, &elements)),
            BulkAccess::Computed(view) => assert!(std::ptr::eq(view, &table)),
            BulkAccess::Compact(view) => assert!(std::ptr::eq(view, &compact)),
            BulkAccess::Positioned(view) => {
                assert!(std::ptr::eq(view.positions, &positions));
                assert!(std::ptr::eq(view.shape, &shape));
                assert!(std::ptr::eq(view.tree.elements, &elements));
            }
        }
    }
}

#[test]
fn metadata_preparation_does_not_eagerly_read_or_refuse_malformed_population() {
    let malformed = Table {
        kinds: vec![CellKind::Number],
        cells: vec![number(4)],
        count: RowCount(2),
    };
    let raw = RepresentationInput::Computed(&malformed);
    let item = registered(RepresentationKind::Computed);
    let contract = item.prepare(raw).unwrap();
    assert_eq!(contract.rows, 2);
    assert!(matches!(
        item.inspect(&contract),
        Err(OperatingError::ShapeMismatch)
    ));
    assert_eq!(
        item.cell(raw, RowIndex(0), ColumnIndex(0)).unwrap(),
        CellRead::Value(number(4))
    );
    let mut decode = [];
    assert!(matches!(
        item.row(raw, RowIndex(1), &mut decode),
        Err(OperatingError::ShapeMismatch)
    ));

    let columns = [column(&[4, 5]), column(&[8])];
    let raw = RepresentationInput::Stored(ColumnsView {
        kinds: &[CellKind::Number; 2],
        columns: &columns,
    });
    let item = registered(RepresentationKind::Stored);
    let contract = item.prepare(raw).unwrap();
    assert_eq!(contract.rows, 2);
    assert_eq!(
        item.inspect(&contract).unwrap(),
        PhysicalShape::ReadablePhysicalFallback
    );
    assert_eq!(
        item.cell(raw, RowIndex(1), ColumnIndex(0)).unwrap(),
        CellRead::Value(number(5))
    );
    let mut decode = [number(99); 2];
    assert!(matches!(
        item.row(raw, RowIndex(1), &mut decode).unwrap(),
        RowRead::MissingPhysical
    ));
}

#[test]
fn selected_projection_order_duplicates_and_source_borrow_do_not_alias() {
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
}

#[test]
fn selected_metadata_and_missing_positions_never_fabricate_rows() {
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
}

#[test]
fn zero_width_population_preserves_each_logical_row_without_inventing_stored_rows() {
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
}

#[test]
fn native_elements_and_edges_preserve_utf8_spans_and_do_not_publish_end_sentinels() {
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
            if form == TreeForm::Elements { 2 } else { 1 }
        );
        let mut decode = [number(0); 5];
        let row = values(item.row(raw, RowIndex(0), &mut decode).unwrap());
        if form == TreeForm::Elements {
            assert_eq!(
                row[4],
                Value::Span(Span {
                    start: Offset(0),
                    end: Offset(2)
                })
            );
        } else {
            assert_eq!(row[0], Value::Node(node_id(FileId(7), Position(0))));
            assert_eq!(row[1], Value::Node(node_id(FileId(7), Position(1))));
        }
        assert!(matches!(
            item.row(raw, RowIndex(contract.rows), &mut decode).unwrap(),
            RowRead::MissingPhysical
        ));
    }
}

#[test]
fn wrong_representation_never_falls_back_to_another_item() {
    let table = Table::default();
    let computed = RepresentationInput::Computed(&table);
    for kind in [
        RepresentationKind::Stored,
        RepresentationKind::Selected,
        RepresentationKind::Tree,
    ] {
        assert!(matches!(
            registered(kind).prepare(computed),
            Err(OperatingError::WrongOperation)
        ));
        assert!(matches!(
            registered(kind).bulk(computed),
            Err(OperatingError::WrongOperation)
        ));
    }
    let stored = RepresentationInput::Stored(ColumnsView {
        kinds: &[],
        columns: &[],
    });
    assert!(matches!(
        registered(RepresentationKind::Computed).prepare(stored),
        Err(OperatingError::WrongOperation)
    ));
    assert!(matches!(
        registered(RepresentationKind::Computed).bulk(stored),
        Err(OperatingError::WrongOperation)
    ));
}

use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};
