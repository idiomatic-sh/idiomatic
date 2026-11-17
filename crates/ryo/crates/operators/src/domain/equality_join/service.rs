//! The engine chooses the physical probe; this kernel owns ordered Join row semantics.
use crate::domain::equality_join::types::JoinShape;
use ryo_values::domain::plan_parts::types::{ColumnIndex, JoinColumn, Side};

/// The one lowering of a join with no closure demand: an inner probe of a `hash_index` of the right rows its filter keeps, each
/// right column read where the view places it.
pub(crate) fn lower(
    shape: JoinShape<'_>,
    filter: &[ryo_values::domain::plan_parts::types::PlanComparison],
    bindings: &[ryo_values::domain::query_bindings::types::ParameterBinding],
) -> super::types::ProbeParts {
    use super::types::{EmptyRight, Existence, ProbeKey, ProbeMode, ProbeParts};
    // No view names every column of the right rows: a column is where it is.
    let placed = |column: &ColumnIndex| match shape
        .view
        .get(usize::try_from(column.0).unwrap_or(usize::MAX))
    {
        Some(physical) => *physical,
        None => *column,
    };
    ProbeParts {
        mode: ProbeMode::Inner(shape.fan),
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

/// The kind of a probe mode: one arm per mode, so a new mode cannot be added without its kind, which `ProbeModeKind::ALL` then
/// holds.
#[cfg(test)]
pub(crate) const fn mode_kind(mode: super::types::ProbeMode) -> super::types::ProbeModeKind {
    use super::types::{ProbeMode, ProbeModeKind};
    match mode {
        ProbeMode::Inner(_) => ProbeModeKind::Inner,
        ProbeMode::Semi => ProbeModeKind::Semi,
        ProbeMode::Anti => ProbeModeKind::Anti,
        ProbeMode::Ordered(_) => ProbeModeKind::Ordered,
    }
}

/// How many right rows a probe of `mode` writes for each left row: a semi, anti or ordered probe keeps at most the left row; an
/// inner probe writes the right rows its proven keys pair.
pub(crate) const fn fan(mode: super::types::ProbeMode) -> crate::domain::growth::types::Fan {
    use super::types::ProbeMode;
    use crate::domain::growth::types::Fan;
    match mode {
        ProbeMode::Semi | ProbeMode::Anti | ProbeMode::Ordered(_) => Fan::PerLeft,
        ProbeMode::Inner(key) => crate::domain::growth::service::fan_of(key),
    }
}
