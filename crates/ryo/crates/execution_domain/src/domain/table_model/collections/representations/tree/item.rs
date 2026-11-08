//! Physical row semantics; policy and allocation ownership remain in the engine.
use crate::domain::table_model::types::{PhysicalShape, RepresentationKind};
use super::super::{interface::RepresentationsItem, types::*};
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::types::RepresentationInput;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;
pub(crate) struct TreeItem;
impl RepresentationsItem for TreeItem {
    fn bulk<'f>(&self, input: RepresentationInput<'f>) -> Result<RepresentationInput<'f>, OperatingError> {
        if matches!(input, RepresentationInput::Tree(_)) {
            Ok(input)
        } else {
            Err(OperatingError::WrongOperation)
        }
    }
    fn prepare<'f>(
        &self,
        input: RepresentationInput<'f>,
    ) -> Result<RelationContract<'f>, OperatingError> {
        let RepresentationInput::Tree(tree) = input else {
            return Err(OperatingError::WrongOperation);
        };
        let relation = match tree.form {
            crate::domain::table_cells::types::TreeForm::Elements => {
                ryo_relations::domain::relations::types::Relation::SyntaxTree
            }
            crate::domain::table_cells::types::TreeForm::Edges => {
                ryo_relations::domain::relations::types::Relation::SyntaxChild
            }
        };
        Ok(RelationContract {
            input,
            kind: RepresentationKind::Tree,
            kinds: ryo_relations::domain::relations::service::kinds(relation),
            rows: RowCount(tree.elements.kind.len().saturating_sub(usize::from(
                tree.form == crate::domain::table_cells::types::TreeForm::Edges,
            ))),
            metadata: Ok(()),
        })
    }
    fn inspect(&self, contract: &RelationContract<'_>) -> Result<PhysicalShape, OperatingError> {
        let RepresentationInput::Tree(tree) = contract.input else {
            return Err(OperatingError::WrongOperation);
        };
        if contract.kinds != contract.input.kinds() {
            return Err(OperatingError::ShapeMismatch);
        }
        let e = tree.elements;
        let n = e.kind.len();
        if e.start.len() != n
            || e.end.len() != n
            || e.text.len() != n
            || e.parent.len() != n
            || e.ordinal.len() != n
            || e.size.len() != n
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
        let mut decode = [Value::Number(ryo_relations::domain::relations::types::Integer(0)); 5];
        Ok(match self.row(input, row, &mut decode)? {
            RowRead::Borrowed(values) | RowRead::Decoded(values) => values
                .get(column.0 as usize)
                .copied()
                .map_or(CellRead::MissingPhysical, CellRead::Value),
            RowRead::MissingPhysical => CellRead::MissingPhysical,
        })
    }
    fn row<'r>(
        &self,
        input: RepresentationInput<'r>,
        row: RowIndex,
        decode: &'r mut [Value],
    ) -> Result<RowRead<'r>, OperatingError> {
        let RepresentationInput::Tree(tree) = input else {
            return Err(OperatingError::WrongOperation);
        };
        if decode.len() < 5 {
            return Err(OperatingError::ShapeMismatch);
        }
        let Some(place) = row
            .0
            .checked_add(usize::from(
                tree.form == crate::domain::table_cells::types::TreeForm::Edges,
            ))
            .and_then(|at| match u32::try_from(at) {
                Ok(place) => Some(place),
                Err(_) => None,
            })
        else {
            return Ok(RowRead::MissingPhysical);
        };
        if place as usize >= tree.elements.kind.len() {
            return Ok(RowRead::MissingPhysical);
        }
        let at = ryo_relations::domain::relations::types::Position(place);
        let values = match tree.form {
            crate::domain::table_cells::types::TreeForm::Elements => {
                crate::domain::table_cells::service::element_of(&tree, at)
            }
            crate::domain::table_cells::types::TreeForm::Edges => {
                crate::domain::table_cells::service::edge_of(&tree, at)
            }
        };
        decode[..5].copy_from_slice(&values);
        Ok(RowRead::Decoded(&decode[..5]))
    }
}

use ryo_values::domain::rows::types::{RowCount, RowIndex};
