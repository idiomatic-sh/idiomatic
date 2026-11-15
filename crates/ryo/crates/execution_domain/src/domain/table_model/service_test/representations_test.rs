//! The registry of physical representations, through its production routes and every item.
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::collections::representations::types::*;
use crate::domain::table_model::service::{borrowed, kind, registered};
use crate::domain::table_model::service_test::{check_rows, column, compact, number};
use crate::domain::table_model::types::RepresentationInput;
use crate::domain::table_model::types::Rows;
use crate::domain::table_model::types::{PhysicalShape, RepresentationKind};
use ryo_language::domain::builders::service::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_operators::domain::tree_cells::types::{TreeForm, TreeRows};
use ryo_relations::domain::relations::constants::NO_SYMBOL;
use ryo_relations::domain::relations::service::{node_id, to_bits};
use ryo_relations::domain::relations::types::{
    CellKind, ColumnsView, FileId, KindBits, Offset, Position, Value as Cell,
};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};

/// A scenario: what a row asks of the part.
type Input = fn() -> Result<Value, Failure>;
/// What a scenario answers: a yes as one.
type Value = usize;
/// Why a scenario is refused.
type Failure = OperatingError;

fn all_registered_representations_preserve_bulk_cells_rows_and_bags() -> Result<Value, Failure> {
    let columns = [column(&[2, 1])];
    let source = ColumnsView {
        kinds: &[CellKind::Number],
        columns: &columns,
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
    for variant in RepresentationKind::ALL {
        // No wildcard: adding a representation requires a real independent fixture.
        let (input, expected) = match variant {
            RepresentationKind::Stored => {
                (Rows::Stored(source), vec![vec![number(2)], vec![number(1)]])
            }
            RepresentationKind::Computed => (
                Rows::Computed(&table),
                vec![vec![number(8)], vec![number(7)]],
            ),
            RepresentationKind::Compact => (
                Rows::Compact(&compact),
                vec![vec![number(8)], vec![number(7)]],
            ),
            RepresentationKind::Tree => (
                Rows::Tree(tree),
                vec![vec![
                    Cell::Node(node_id(FileId(3), Position(0))),
                    Cell::Node(node_id(FileId(3), Position(1))),
                    number(0),
                    Cell::Symbol(NO_SYMBOL),
                    Cell::File(FileId(3)),
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
            RepresentationInput::Stored(view) => {
                assert!(std::ptr::eq(view.columns, source.columns))
            }
            RepresentationInput::Tree(view) => assert!(std::ptr::eq(view.elements, &elements)),
            RepresentationInput::Computed(view) => assert!(std::ptr::eq(view, &table)),
            RepresentationInput::Compact(view) => assert!(std::ptr::eq(view, &compact)),
        }
    }
    Ok(1)
}

fn metadata_preparation_does_not_eagerly_read_or_refuse_malformed_population()
-> Result<Value, Failure> {
    let malformed = Table {
        kinds: vec![CellKind::Number],
        cells: vec![number(4)],
        count: RowCount(2),
    };
    let raw = RepresentationInput::Computed(&malformed);
    let item = registered(RepresentationKind::Computed);
    let contract = item.prepare(raw).unwrap();
    assert_eq!(contract.rows, RowCount(2));
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
    assert_eq!(contract.rows, RowCount(2));
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
    Ok(1)
}

fn wrong_representation_never_falls_back_to_another_item() -> Result<Value, Failure> {
    let table = Table::default();
    let computed = RepresentationInput::Computed(&table);
    for kind in [RepresentationKind::Stored, RepresentationKind::Tree] {
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
    Ok(1)
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("all_registered_representations_preserve_bulk_cells_rows_and_bags"),
        word: Word::Succeeds,
        input: || all_registered_representations_preserve_bulk_cells_rows_and_bags,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("metadata_preparation_does_not_eagerly_read_or_refuse_malformed_population"),
        word: Word::Succeeds,
        input: || metadata_preparation_does_not_eagerly_read_or_refuse_malformed_population,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("wrong_representation_never_falls_back_to_another_item"),
        word: Word::Succeeds,
        input: || wrong_representation_never_falls_back_to_another_item,
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
