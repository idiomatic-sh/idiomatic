//! Pure coordinate adaptation of already-admitted borrowed row representations.
use super::types::{PositionInput, PositionWork};
use crate::domain::{
    lowering::types::{LoweredColumn, LoweredShape, RowPosition, RowWidth},
    operating::{errors::OperatingError, types::OperatorContext},
    operator::contract::{
        derived::ScopedDerived,
        types::{PrimitiveSemantics, WorkCounters},
    },
    positions::types::{Positions, Slots},
    table_cells::types::TreeForm,
    table_model::types::Rows,
};
use ryo_values::domain::plans::types::Operation;
pub(crate) fn evaluate(
    semantics: &dyn PrimitiveSemantics,
    derived: &mut dyn ScopedDerived,
    operation: &Operation,
    rows: &[Rows<'_>],
    shape: &LoweredShape,
    context: &mut OperatorContext<'_>,
    work: &mut PositionWork,
    counts: &WorkCounters,
) -> Result<Option<Positions>, OperatingError> {
    if rows
        .iter()
        .any(|rows| !matches!(rows, Rows::Tree(_) | Rows::Positioned(_)))
    {
        return Ok(None);
    }
    let Some(tree) = rows.first().and_then(|rows| match rows {
        Rows::Tree(tree) => Some(*tree),
        Rows::Positioned(view) => Some(view.tree),
        _ => None,
    }) else {
        return Ok(None);
    };
    let mut direct = Vec::new();
    for rows in rows {
        match rows {
            Rows::Tree(each) => {
                if !std::ptr::eq(each.elements, tree.elements) || each.file != tree.file {
                    return Err(OperatingError::NotRead);
                }
                let fields = match each.form {
                    TreeForm::Elements => &crate::domain::lowering::constants::ELEMENT,
                    TreeForm::Edges => &crate::domain::lowering::constants::EDGE,
                };
                let shape = LoweredShape {
                    width: RowWidth(1),
                    columns: fields
                        .iter()
                        .map(|field| LoweredColumn {
                            slot: RowPosition(0),
                            field: *field,
                        })
                        .collect(),
                };
                let start = usize::from(each.form == TreeForm::Edges);
                let end = ryo_language::domain::elements::service::real_count(each.elements).0;
                let slots = (start..end)
                    .map(u32::try_from)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| OperatingError::ShapeMismatch)?;
                direct.push(Some((
                    shape,
                    Positions {
                        width: RowWidth(1),
                        slots: Slots(slots),
                    },
                )));
            }
            Rows::Positioned(view) => {
                if !std::ptr::eq(view.tree.elements, tree.elements) || view.tree.file != tree.file {
                    return Err(OperatingError::NotRead);
                }
                direct.push(None);
            }
            _ => return Err(OperatingError::WrongOperation),
        }
    }
    let inputs: Vec<_> = rows
        .iter()
        .zip(&direct)
        .map(|(rows, direct)| match (rows, direct) {
            (Rows::Positioned(view), None) => Ok(PositionInput {
                shape: view.shape,
                rows: view.positions,
            }),
            (_, Some((shape, positions))) => Ok(PositionInput {
                shape,
                rows: positions,
            }),
            _ => Err(OperatingError::WrongOperation),
        })
        .collect::<Result<_, _>>()?;
    let result = crate::domain::native_positions::service::evaluate(
        semantics, derived, operation, &tree, &inputs, shape, context, work, counts,
    );
    result.map(Some)
}
