//! Physical row semantics; policy and allocation ownership remain in the engine.
use crate::domain::table_model::types::{PhysicalShape, RelationContract, RepresentationKind};
use super::super::{interface::RepresentationsItem, types::*};
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::types::RepresentationInput;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;
pub(crate) struct StoredItem;
impl RepresentationsItem for StoredItem {
    fn bulk<'f>(&self, input: RepresentationInput<'f>) -> Result<RepresentationInput<'f>, OperatingError> {
        if matches!(input, RepresentationInput::Stored(_)) {
            Ok(input)
        } else {
            Err(OperatingError::WrongOperation)
        }
    }
    fn prepare<'f>(
        &self,
        input: RepresentationInput<'f>,
    ) -> Result<RelationContract<'f>, OperatingError> {
        let RepresentationInput::Stored(view) = input else {
            return Err(OperatingError::WrongOperation);
        };
        Ok(RelationContract {
            input,
            kind: RepresentationKind::Stored,
            kinds: view.kinds,
            rows: RowCount(view.columns.first().map_or(0, |c| c.0.len())),
            metadata: if view.columns.len() == view.kinds.len() {
                Ok(())
            } else {
                Err(OperatingError::ShapeMismatch)
            },
        })
    }
    fn inspect(&self, contract: &RelationContract<'_>) -> Result<PhysicalShape, OperatingError> {
        contract.metadata.as_ref().map_err(Clone::clone)?;
        let RepresentationInput::Stored(view) = contract.input else {
            return Err(OperatingError::WrongOperation);
        };
        if contract.kinds != contract.input.kinds() {
            return Err(OperatingError::ShapeMismatch);
        }
        Ok(if view.columns.iter().all(|c| c.0.len() == contract.rows.0) {
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
        let RepresentationInput::Stored(view) = input else {
            return Err(OperatingError::WrongOperation);
        };
        let at = column.0 as usize;
        Ok(
            match view
                .kinds
                .get(at)
                .zip(view.columns.get(at).and_then(|c| c.0.get(row.0)))
            {
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
        let RepresentationInput::Stored(view) = input else {
            return Err(OperatingError::WrongOperation);
        };
        if decode.len() < view.kinds.len() {
            return Err(OperatingError::ShapeMismatch);
        }
        for (at, kind) in view.kinds.iter().enumerate() {
            let Some(bits) = view.columns.get(at).and_then(|c| c.0.get(row.0)) else {
                return Ok(RowRead::MissingPhysical);
            };
            decode[at] = ryo_relations::domain::relations::service::from_bits(
                *kind,
                ryo_relations::domain::relations::types::Bits(*bits),
            );
        }
        Ok(RowRead::Decoded(&decode[..view.kinds.len()]))
    }
}

use ryo_values::domain::rows::types::{RowCount, RowIndex};
