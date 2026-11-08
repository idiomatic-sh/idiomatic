//! Physical row semantics; policy and allocation ownership remain in the engine.
use crate::domain::table_model::types::{PhysicalShape, RelationContract, RepresentationKind};
use super::super::{interface::RepresentationsItem, types::*};
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::types::RepresentationInput;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;
pub(crate) struct ComputedItem;
impl RepresentationsItem for ComputedItem {
    fn bulk<'f>(&self, input: RepresentationInput<'f>) -> Result<RepresentationInput<'f>, OperatingError> {
        if matches!(input, RepresentationInput::Computed(_)) {
            Ok(input)
        } else {
            Err(OperatingError::WrongOperation)
        }
    }
    fn prepare<'f>(
        &self,
        input: RepresentationInput<'f>,
    ) -> Result<RelationContract<'f>, OperatingError> {
        let RepresentationInput::Computed(table) = input else {
            return Err(OperatingError::WrongOperation);
        };
        Ok(RelationContract {
            input,
            kind: RepresentationKind::Computed,
            kinds: &table.kinds,
            rows: table.count,
            metadata: Ok(()),
        })
    }
    fn inspect(&self, contract: &RelationContract<'_>) -> Result<PhysicalShape, OperatingError> {
        let RepresentationInput::Computed(table) = contract.input else {
            return Err(OperatingError::WrongOperation);
        };
        if contract.kinds != contract.input.kinds() {
            return Err(OperatingError::ShapeMismatch);
        }
        if table.count.0.checked_mul(table.kinds.len()) != Some(table.cells.len()) {
            return Err(OperatingError::ShapeMismatch);
        }
        Ok(PhysicalShape::Complete)
    }
    fn cell(
        &self,
        input: RepresentationInput<'_>,
        row: RowIndex,
        column: ColumnIndex,
    ) -> Result<CellRead, OperatingError> {
        let RepresentationInput::Computed(table) = input else {
            return Err(OperatingError::WrongOperation);
        };
        if row.0 >= table.count.0 || (column.0 as usize) >= table.kinds.len() {
            return Ok(CellRead::MissingPhysical);
        }
        let offset = row
            .0
            .checked_mul(table.kinds.len())
            .and_then(|at| at.checked_add(column.0 as usize));
        Ok(offset
            .and_then(|at| table.cells.get(at))
            .copied()
            .map_or(CellRead::MissingPhysical, CellRead::Value))
    }
    fn row<'r>(
        &self,
        input: RepresentationInput<'r>,
        row: RowIndex,
        _decode: &'r mut [Value],
    ) -> Result<RowRead<'r>, OperatingError> {
        let RepresentationInput::Computed(table) = input else {
            return Err(OperatingError::WrongOperation);
        };
        if row.0 >= table.count.0 {
            return Ok(RowRead::MissingPhysical);
        }
        let begin = row
            .0
            .checked_mul(table.kinds.len())
            .ok_or(OperatingError::ShapeMismatch)?;
        let end = begin
            .checked_add(table.kinds.len())
            .ok_or(OperatingError::ShapeMismatch)?;
        table
            .cells
            .get(begin..end)
            .map(RowRead::Borrowed)
            .ok_or(OperatingError::ShapeMismatch)
    }
}

use ryo_values::domain::rows::types::RowIndex;
