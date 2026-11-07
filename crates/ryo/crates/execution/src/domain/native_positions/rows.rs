//! Pure coordinate adaptation of already-admitted borrowed row representations.
use super::{
    keys,
    types::{DirectPositions, PositionInput, PositionWork, RawKeys},
};
use crate::domain::{
    lowering::types::{LoweredColumn, LoweredShape, RowPosition, RowWidth},
    operating::{errors::OperatingError, types::OperatorContext},
    operator::contract::types::{PrimitiveSemantics, WorkCounters},
    positions::types::{Positions, Slots},
    table_cells::types::{TreeForm, TreeRows},
    table_model::types::Rows,
};
use ryo_derived::domain::raw_order::types::raw_order::value::RawOrder;
use ryo_values::domain::plans::types::Operation;
/// Source rows adapted to coordinates once, so the key phase and the evaluation phase share them.
pub(crate) struct Staged<'s, 'a> {
    tree: TreeRows<'a>,
    rows: &'s [Rows<'a>],
    direct: Vec<Option<DirectPositions>>,
}

/// Adapt the source rows; none when they are not tree rows this module can read.
pub(crate) fn stage<'s, 'a>(rows: &'s [Rows<'a>]) -> Result<Option<Staged<'s, 'a>>, OperatingError> {
    let Some(tree) = source_tree(rows)? else {
        return Ok(None);
    };
    let mut direct = Vec::new();
    for rows in rows {
        match rows {
            Rows::Tree(each) => {
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
                direct.push(Some(DirectPositions {
                    shape,
                    positions: Positions {
                        width: RowWidth(1),
                        slots: Slots(slots),
                    },
                }));
            }
            Rows::Positioned(_) => {
                direct.push(None);
            }
            _ => return Err(OperatingError::WrongOperation),
        }
    }
    Ok(Some(Staged { tree, rows, direct }))
}

impl Staged<'_, '_> {
    fn inputs(&self) -> Result<Vec<PositionInput<'_>>, OperatingError> {
        self.rows
        .iter()
            .zip(&self.direct)
        .map(|(rows, direct)| match (rows, direct) {
            (Rows::Positioned(view), None) => Ok(PositionInput {
                shape: view.shape,
                rows: view.positions,
            }),
            (_, Some(DirectPositions { shape, positions })) => Ok(PositionInput {
                shape,
                rows: positions,
            }),
            _ => Err(OperatingError::WrongOperation),
        })
            .collect::<Result<_, _>>()
    }

    /// Keys the operation needs ordered, or none when it needs no raw order.
    pub(crate) fn raw_keys(
        &self,
        operation: &Operation,
        shape: &LoweredShape,
        work: &mut PositionWork,
        counts: &WorkCounters,
    ) -> Result<Option<RawKeys>, OperatingError> {
        let inputs = self.inputs()?;
        keys::raw_keys(operation, &self.tree, &inputs, shape, work, counts)
    }

    /// Evaluate with the raw order the application acquired for `raw_keys`.
    pub(crate) fn evaluate(
        &self,
        semantics: &dyn PrimitiveSemantics,
        order: Option<&RawOrder>,
        operation: &Operation,
        shape: &LoweredShape,
        context: &mut OperatorContext<'_>,
        work: &mut PositionWork,
        counts: &WorkCounters,
    ) -> Result<Positions, OperatingError> {
        let inputs = self.inputs()?;
        crate::domain::native_positions::service::evaluate(
            semantics, order, operation, &self.tree, &inputs, shape, context, work, counts,
        )
    }
}

/// The same source compatibility check serves ordinary evaluation and an empty
/// native publication; an empty primary cannot hide a foreign tree input.
pub(crate) fn source_tree<'a>(
    rows: &[Rows<'a>],
) -> Result<Option<crate::domain::table_cells::types::TreeRows<'a>>, OperatingError> {
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
    for rows in rows {
        let each = match rows {
            Rows::Tree(tree) => *tree,
            Rows::Positioned(view) => view.tree,
            _ => return Ok(None),
        };
        if !std::ptr::eq(each.elements, tree.elements) || each.file != tree.file {
            return Err(OperatingError::NotRead);
        }
    }
    Ok(Some(tree))
}
