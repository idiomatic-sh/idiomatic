//! Physical row semantics; policy and allocation ownership remain in the engine.
use super::super::{interface::RepresentationsItem, types::*};
use crate::domain::{operating::errors::OperatingError, table_cells::types::RowIndex};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;
pub(crate) struct SelectedItem;
impl RepresentationsItem for SelectedItem {
    fn bulk<'f>(&self, input: RepresentationInput<'f>) -> Result<BulkAccess<'f>, OperatingError> {
        if matches!(input, RepresentationInput::Selected { .. }) {
            Ok(input)
        } else {
            Err(OperatingError::WrongOperation)
        }
    }
    fn prepare<'f>(
        &self,
        input: RepresentationInput<'f>,
    ) -> Result<RelationContract<'f>, OperatingError> {
        let RepresentationInput::Selected {
            source,
            positions,
            columns,
            kinds,
        } = input
        else {
            return Err(OperatingError::WrongOperation);
        };
        let metadata = if columns.len() == kinds.len()
            && columns
                .iter()
                .zip(kinds)
                .all(|(column, kind)| source.kinds.get(column.0 as usize) == Some(kind))
        {
            Ok(())
        } else {
            Err(OperatingError::ShapeMismatch)
        };
        Ok(RelationContract {
            input,
            kind: RepresentationKind::Selected,
            kinds,
            rows: positions.len(),
            metadata,
        })
    }
    fn inspect(&self, contract: &RelationContract<'_>) -> Result<PhysicalShape, OperatingError> {
        contract.metadata.as_ref().map_err(Clone::clone)?;
        let RepresentationInput::Selected {
            source,
            positions,
            columns,
            ..
        } = contract.input
        else {
            return Err(OperatingError::WrongOperation);
        };
        if contract.kinds != contract.input.kinds() {
            return Err(OperatingError::ShapeMismatch);
        }
        let n = source.columns.first().map_or(0, |c| c.0.len());
        let rectangular = source.columns.len() == source.kinds.len()
            && source.columns.iter().all(|c| c.0.len() == n);
        let complete = positions.iter().all(|position| {
            columns.iter().all(|column| {
                source
                    .columns
                    .get(column.0 as usize)
                    .is_some_and(|values| (*position as usize) < values.0.len())
            })
        });
        Ok(if rectangular && complete {
            PhysicalShape::Complete
        } else {
            PhysicalShape::ReadablePhysicalFallback
        })
    }
    fn cell(
        &self,
        input: RepresentationInput<'_>,
        row: RowIndex,
        column: ColumnIndex,
    ) -> Result<CellRead, OperatingError> {
        let RepresentationInput::Selected {
            source,
            positions,
            columns,
            ..
        } = input
        else {
            return Err(OperatingError::WrongOperation);
        };
        let Some((physical, column)) = positions.get(row.0).zip(columns.get(column.0 as usize))
        else {
            return Ok(CellRead::MissingPhysical);
        };
        let at = column.0 as usize;
        Ok(
            match source.kinds.get(at).zip(
                source
                    .columns
                    .get(at)
                    .and_then(|c| c.0.get(*physical as usize)),
            ) {
                Some((kind, bits)) => {
                    CellRead::Value(ryo_relations::domain::relations::service::from_bits(
                        *kind,
                        ryo_relations::domain::relations::types::Bits(*bits),
                    ))
                }
                None => CellRead::MissingPhysical,
            },
        )
    }
    fn row<'r>(
        &self,
        input: RepresentationInput<'r>,
        row: RowIndex,
        decode: &'r mut [Value],
    ) -> Result<RowRead<'r>, OperatingError> {
        let RepresentationInput::Selected {
            positions, columns, ..
        } = input
        else {
            return Err(OperatingError::WrongOperation);
        };
        if row.0 >= positions.len() {
            return Ok(RowRead::MissingPhysical);
        }
        if decode.len() < columns.len() {
            return Err(OperatingError::ShapeMismatch);
        }
        for (at, _) in columns.iter().enumerate() {
            match self.cell(input, row, ColumnIndex(at as u32))? {
                CellRead::Value(value) => decode[at] = value,
                CellRead::MissingPhysical => return Ok(RowRead::MissingPhysical),
            }
        }
        Ok(RowRead::Decoded(&decode[..columns.len()]))
    }
}
