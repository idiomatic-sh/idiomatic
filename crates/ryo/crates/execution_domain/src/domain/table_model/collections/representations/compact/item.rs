//! Physical row semantics; policy and allocation ownership remain in the engine.
use crate::domain::table_model::types::{PhysicalShape, RelationContract, RepresentationKind};
use super::super::{interface::RepresentationsItem, types::*};
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::types::RepresentationInput;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;
pub(crate) struct CompactItem;
impl RepresentationsItem for CompactItem {
    fn bulk<'f>(&self, input: RepresentationInput<'f>) -> Result<RepresentationInput<'f>, OperatingError> {
        if matches!(input, RepresentationInput::Compact(_)) {
            Ok(input)
        } else {
            Err(OperatingError::WrongOperation)
        }
    }
    fn prepare<'f>(
        &self,
        input: RepresentationInput<'f>,
    ) -> Result<RelationContract<'f>, OperatingError> {
        let RepresentationInput::Compact(table) = input else {
            return Err(OperatingError::WrongOperation);
        };
        Ok(RelationContract {
            input,
            kind: RepresentationKind::Compact,
            kinds: ryo_values::domain::compact_table::service::compact_kinds(table),
            rows: ryo_values::domain::compact_table::service::compact_rows(table),
            metadata: Ok(()),
        })
    }
    fn inspect(&self, contract: &RelationContract<'_>) -> Result<PhysicalShape, OperatingError> {
        let RepresentationInput::Compact(table) = contract.input else {
            return Err(OperatingError::WrongOperation);
        };
        if contract.kinds != contract.input.kinds() {
            return Err(OperatingError::ShapeMismatch);
        }
        if ryo_values::domain::compact_table::service::compact_rectangular(table)
            != ryo_values::domain::table_words::types::TableShape::Rectangular
        {
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
        let RepresentationInput::Compact(table) = input else {
            return Err(OperatingError::WrongOperation);
        };
        if row.0 >= ryo_values::domain::compact_table::service::compact_rows(table).0
            || column.0 as usize
                >= ryo_values::domain::compact_table::service::compact_kinds(table).len()
        {
            return Ok(CellRead::MissingPhysical);
        }
        ryo_values::domain::compact_table::service::compact_cell(table, row, column)
            .map(CellRead::Value)
            .ok_or(OperatingError::ShapeMismatch)
    }
    fn row<'r>(
        &self,
        input: RepresentationInput<'r>,
        row: RowIndex,
        _decode: &'r mut [Value],
    ) -> Result<RowRead<'r>, OperatingError> {
        let RepresentationInput::Compact(table) = input else {
            return Err(OperatingError::WrongOperation);
        };
        if row.0 >= ryo_values::domain::compact_table::service::compact_rows(table).0 {
            return Ok(RowRead::MissingPhysical);
        }
        ryo_values::domain::compact_table::service::compact_read(table, row, _decode)
            .map(RowRead::Decoded)
            .ok_or(OperatingError::ShapeMismatch)
    }
}
use ryo_values::domain::rows::types::RowIndex;
