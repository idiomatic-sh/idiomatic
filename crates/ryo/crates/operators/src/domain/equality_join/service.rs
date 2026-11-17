//! The engine chooses the physical probe; this kernel owns ordered Join row semantics.
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::equality_join::types::{JoinShape, ProbeMode, ProbeParts};
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::{ColumnIndex, JoinColumn, Side};

/// The one lowering of a join with no closure demand: an inner probe of a `hash_index` of the right rows its filter keeps, each
/// right column read where the view places it.
pub(crate) fn lower(
    shape: JoinShape<'_>,
    filter: &[ryo_values::domain::plan_parts::types::PlanComparison],
    bindings: &[ryo_values::domain::query_bindings::types::ParameterBinding],
) -> ProbeParts {
    use super::types::{EmptyRight, Existence, ProbeKey};
    // No view names every column of the right rows: a column is where it is.
    let placed = |column: &ColumnIndex| match shape
        .view
        .get(usize::try_from(column.0).unwrap_or(usize::MAX))
    {
        Some(physical) => *physical,
        None => *column,
    };
    ProbeParts {
        mode: ProbeMode::Inner,
        key: ProbeKey::Columns {
            left: shape.left_on.to_vec(),
            right: shape.right_on.iter().map(placed).collect(),
        },
        filter: crate::domain::select::service::lower(filter, &[], bindings),
        output: shape
            .output
            .iter()
            .map(|column| match column.side {
                Side::Left => *column,
                Side::Right => JoinColumn {
                    side: Side::Right,
                    column: placed(&column.column),
                },
            })
            .collect(),
        existence: Existence::Every,
        repeated: ryo_values::domain::plan_parts::types::RepeatedRows::Kept,
        empty_right: EmptyRight::KeyRead,
    }
}

/// The join a closure was fused into, over the rows its seeded walk reaches: the probe of those rows by the left rows, and
/// the logical right row each reached entity stands for, over the partition's keys, then the source, then the entity.
pub(crate) fn seeded(
    shape: JoinShape<'_>,
    filter: &[ryo_values::domain::plan_parts::types::PlanComparison],
    bindings: &[ryo_values::domain::query_bindings::types::ParameterBinding],
) -> super::types::SeededJoin {
    super::types::SeededJoin {
        columns: shape
            .reach
            .map(|reach| reach.columns.clone())
            .unwrap_or_default(),
        probe: lower(shape, filter, bindings),
    }
}

/// The kinds of a probe's answer over left rows of `left` and right rows of `right`.
///
/// # Errors
///
/// `NoSuchColumn` for an answer column past its side's rows.
pub(crate) fn answer_kinds(
    probe: &ProbeParts,
    left: &[CellKind],
    right: &[CellKind],
) -> Result<Vec<CellKind>, PortableOperatorFault> {
    if probe.mode != ProbeMode::Inner {
        return Ok(left.to_vec());
    }
    probe
        .output
        .iter()
        .map(|column| {
            let side = match column.side {
                Side::Left => left,
                Side::Right => right,
            };
            side.get(usize::try_from(column.column.0).unwrap_or(usize::MAX))
                .copied()
                .ok_or(PortableOperatorFault::NoSuchColumn {
                    index: column.column,
                })
        })
        .collect()
}
