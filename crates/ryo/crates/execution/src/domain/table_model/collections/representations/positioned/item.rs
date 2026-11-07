//! Pure row access over immutable positions and their admitted tree geometry.
use super::super::{interface::RepresentationsItem, types::*};
use crate::domain::{operating::errors::OperatingError, table_model::types::PositionedRows};
use ryo_relations::domain::relations::types::{Position, Value};
use ryo_values::domain::{
    plan_parts::types::ColumnIndex,
    rows::types::{RowCount, RowIndex},
};
pub(crate) struct PositionedItem;
fn count(view: PositionedRows<'_>) -> Result<RowCount, OperatingError> {
    let width = view.positions.width.0;
    if width == 0 || width != view.shape.width.0 || view.positions.slots.0.len() % width != 0 {
        return Err(OperatingError::ShapeMismatch);
    }
    Ok(RowCount(view.positions.slots.0.len() / width))
}
impl RepresentationsItem for PositionedItem {
    fn prepare<'f>(
        &self,
        input: RepresentationInput<'f>,
    ) -> Result<RelationContract<'f>, OperatingError> {
        let RepresentationInput::Positioned(view) = input else {
            return Err(OperatingError::WrongOperation);
        };
        Ok(RelationContract {
            input,
            kind: RepresentationKind::Positioned,
            kinds: view.kinds,
            rows: count(view)?,
            metadata: Ok(()),
        })
    }
    fn inspect(&self, contract: &RelationContract<'_>) -> Result<PhysicalShape, OperatingError> {
        let RepresentationInput::Positioned(view) = contract.input else {
            return Err(OperatingError::WrongOperation);
        };
        count(view)?;
        if contract.kinds != view.kinds
            || view.shape.columns.len() != view.kinds.len()
            || view
                .shape
                .columns
                .iter()
                .zip(view.kinds)
                .any(|(column, kind)| {
                    column.slot.0 >= view.positions.width.0
                        || crate::domain::lowering::service::kind_of(column.field) != *kind
                })
            || view
                .positions
                .slots
                .0
                .iter()
                .any(|at| *at as usize >= view.tree.elements.kind.len())
        {
            return Err(OperatingError::ShapeMismatch);
        }
        let tree = RepresentationInput::Tree(view.tree);
        let source = super::super::tree::item::TreeItem.prepare(tree)?;
        super::super::tree::item::TreeItem.inspect(&source)?;
        Ok(PhysicalShape::Complete)
    }
    fn bulk<'f>(&self, input: RepresentationInput<'f>) -> Result<RepresentationInput<'f>, OperatingError> {
        if matches!(input, RepresentationInput::Positioned(_)) {
            Ok(input)
        } else {
            Err(OperatingError::WrongOperation)
        }
    }
    fn cell(
        &self,
        input: RepresentationInput<'_>,
        row: RowIndex,
        column: ColumnIndex,
    ) -> Result<CellRead, OperatingError> {
        let RepresentationInput::Positioned(view) = input else {
            return Err(OperatingError::WrongOperation);
        };
        let Some(column) = view.shape.columns.get(column.0 as usize) else {
            return Ok(CellRead::MissingPhysical);
        };
        if row.0 >= count(view)?.0 || column.slot.0 >= view.positions.width.0 {
            return Ok(CellRead::MissingPhysical);
        }
        let offset = row
            .0
            .checked_mul(view.positions.width.0)
            .and_then(|base| base.checked_add(column.slot.0))
            .ok_or(OperatingError::ShapeMismatch)?;
        let Some(at) = view.positions.slots.0.get(offset).copied() else {
            return Ok(CellRead::MissingPhysical);
        };
        let e = view.tree.elements;
        let safe = [
            e.kind.len(),
            e.start.len(),
            e.end.len(),
            e.text.len(),
            e.parent.len(),
            e.ordinal.len(),
            e.size.len(),
        ]
        .into_iter()
        .min()
        .ok_or(OperatingError::ShapeMismatch)?;
        if at as usize >= safe {
            return Ok(CellRead::MissingPhysical);
        }
        Ok(CellRead::Value(crate::domain::positions::service::value(
            &view.tree,
            column.field,
            Position(at),
        )))
    }
    fn row<'a>(
        &self,
        input: RepresentationInput<'a>,
        row: RowIndex,
        decode: &'a mut [Value],
    ) -> Result<RowRead<'a>, OperatingError> {
        let RepresentationInput::Positioned(view) = input else {
            return Err(OperatingError::WrongOperation);
        };
        if row.0 >= count(view)?.0 {
            return Ok(RowRead::MissingPhysical);
        }
        if decode.len() < view.kinds.len() || view.shape.columns.len() != view.kinds.len() {
            return Err(OperatingError::ShapeMismatch);
        }
        for (at, slot) in decode.iter_mut().take(view.kinds.len()).enumerate() {
            match self.cell(
                input,
                row,
                ColumnIndex(u32::try_from(at).map_err(|_| OperatingError::ShapeMismatch)?),
            )? {
                CellRead::Value(value) => *slot = value,
                CellRead::MissingPhysical => return Ok(RowRead::MissingPhysical),
            }
        }
        Ok(RowRead::Decoded(&decode[..view.kinds.len()]))
    }
}
