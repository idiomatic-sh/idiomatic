//! The engine chooses the physical probe; this kernel owns ordered Join row semantics, and the band join's one lowering.
use crate::domain::equality_join::types::JoinShape;
use ryo_values::domain::band_joins::types::{BandAnswer, BandEdge};
use ryo_values::domain::plan_parts::service::view_source;
use ryo_values::domain::plan_parts::types::{ColumnIndex, JoinColumn, KeyFan, Side};

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

/// What a band join is, written in the plan's columns: every right column read through `view`. None when a right column is
/// past the view, or when the band is empty.
#[must_use]
pub(crate) fn banded(
    keys: [&[ColumnIndex]; 2],
    band: &[BandEdge],
    answer: BandAnswer,
    output: &[JoinColumn],
    view: &[ColumnIndex],
    fan: KeyFan,
) -> Option<super::types::BandParts> {
    let [left_on, right_on] = keys;
    let placed = |column: ColumnIndex| view_source(view, column);
    if band.is_empty() {
        return None;
    }
    Some(super::types::BandParts {
        left_on: left_on.to_vec(),
        right_on: right_on
            .iter()
            .map(|key| placed(*key))
            .collect::<Option<_>>()?,
        band: band
            .iter()
            .map(|edge| {
                placed(edge.right).map(|right| BandEdge {
                    left: edge.left,
                    test: edge.test,
                    right,
                })
            })
            .collect::<Option<_>>()?,
        answer,
        output: output
            .iter()
            .map(|column| match column.side {
                Side::Left => Some(*column),
                Side::Right => placed(column.column).map(|right| JoinColumn {
                    side: Side::Right,
                    column: right,
                }),
            })
            .collect::<Option<_>>()?,
        fan,
    })
}

/// How many right rows a band join answers for each left row: the nearest are the ties of one row; every row of the band is
/// what its proven keys pair.
#[must_use]
pub(crate) const fn band_fan(parts: &super::types::BandParts) -> crate::domain::growth::types::Fan {
    match parts.answer {
        BandAnswer::Nearest => crate::domain::growth::types::Fan::PerLeft,
        BandAnswer::Every => crate::domain::growth::service::fan_of(parts.fan),
    }
}

/// The comparison an edge makes, as `right test left`: the one mapping from a band's test to the language's.
#[must_use]
pub(crate) const fn test_of(
    test: ryo_values::domain::band_joins::types::BandTest,
) -> ryo_values::domain::plan_parts::types::Test {
    use ryo_values::domain::band_joins::types::BandTest;
    use ryo_values::domain::plan_parts::types::Test;
    match test {
        BandTest::Below => Test::LessThan,
        BandTest::AtOrBelow => Test::LessThanOrEqual,
        BandTest::Above => Test::GreaterThan,
        BandTest::AtOrAbove => Test::GreaterThanOrEqual,
    }
}
