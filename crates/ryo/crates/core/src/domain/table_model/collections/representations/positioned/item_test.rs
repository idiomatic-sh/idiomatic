//! The actual item validates tuple geometry before exposing borrowed cells.
use super::super::{
    interface::RepresentationsItem,
    types::{CellRead, PhysicalShape, RepresentationInput},
};
use super::item::PositionedItem;
use crate::domain::{
    lowering::types::{ElementPart, LoweredColumn, LoweredShape, RowPosition, RowWidth},
    operating::errors::OperatingError,
    positions::types::{Positions, Slots},
    table_cells::types::{TreeForm, TreeRows},
    table_model::types::PositionedRows,
};
use ryo_language::domain::{
    elements::{
        service::{enter_node, leave_node, table_builder},
        types::Elements,
    },
    symbols::types::{ByteSlice, Symbols},
};
use ryo_relations::domain::relations::types::{CellKind, FileId, KindBits, Offset, Value};
use ryo_values::domain::{plan_parts::types::ColumnIndex, rows::types::RowIndex};
fn input<'a>(
    tree: TreeRows<'a>,
    positions: &'a Positions,
    shape: &'a LoweredShape,
) -> RepresentationInput<'a> {
    RepresentationInput::Positioned(PositionedRows {
        tree,
        positions,
        shape,
        kinds: &[CellKind::File],
    })
}
#[test]
fn tuple_geometry_preserves_bags_and_refuses_invalid_coordinates() {
    let mut elements = Elements::default();
    let mut symbols = Symbols::default();
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b""));
    enter_node(&mut builder, KindBits(0), Offset(0));
    leave_node(&mut builder, Offset(0));
    let tree = TreeRows {
        elements: &elements,
        file: FileId(7),
        form: TreeForm::Elements,
    };
    let mut positions = Positions {
        width: RowWidth(1),
        slots: Slots(vec![0, 0]),
    };
    let mut shape = LoweredShape {
        width: RowWidth(1),
        columns: vec![LoweredColumn {
            slot: RowPosition(0),
            field: ElementPart::File,
        }],
    };
    let contract = PositionedItem
        .prepare(input(tree, &positions, &shape))
        .unwrap();
    assert_eq!(
        PositionedItem.inspect(&contract),
        Ok(PhysicalShape::Complete)
    );
    assert_eq!(contract.rows, 2);
    for row in [RowIndex(0), RowIndex(1)] {
        assert_eq!(
            PositionedItem.cell(contract.input, row, ColumnIndex(0)),
            Ok(CellRead::Value(Value::File(FileId(7))))
        );
    }
    positions.slots.0.push(1);
    let contract = PositionedItem
        .prepare(input(tree, &positions, &shape))
        .unwrap();
    assert_eq!(
        PositionedItem.inspect(&contract),
        Err(OperatingError::ShapeMismatch)
    );
    positions.slots.0.pop();
    shape.columns[0].slot = RowPosition(1);
    let contract = PositionedItem
        .prepare(input(tree, &positions, &shape))
        .unwrap();
    assert_eq!(
        PositionedItem.inspect(&contract),
        Err(OperatingError::ShapeMismatch)
    );
    positions.width = RowWidth(0);
    assert!(matches!(
        PositionedItem.prepare(input(tree, &positions, &shape)),
        Err(OperatingError::ShapeMismatch)
    ));
}
