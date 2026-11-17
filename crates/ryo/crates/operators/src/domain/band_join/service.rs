//! The `band_join` service: a band join's one lowering, and the fan it answers.
use super::types::BandParts;
use crate::domain::growth::types::Fan;
use ryo_values::domain::plan_parts::service::view_source;
use ryo_values::domain::plan_parts::types::{
    BandAnswer, BandEdge, ColumnIndex, JoinColumn, KeyFan, Side,
};

/// What a band join is, written in the plan's columns: every right column read through `view`. None when a right column is
/// past the view, or when the band is empty.
#[must_use]
pub(crate) fn lower(
    keys: [&[ColumnIndex]; 2],
    band: &[BandEdge],
    answer: BandAnswer,
    output: &[JoinColumn],
    view: &[ColumnIndex],
    fan: KeyFan,
) -> Option<BandParts> {
    let [left_on, right_on] = keys;
    let placed = |column: ColumnIndex| view_source(view, column);
    if band.is_empty() {
        return None;
    }
    Some(BandParts {
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
pub(crate) const fn fan(parts: &BandParts) -> Fan {
    match parts.answer {
        BandAnswer::Nearest => Fan::PerLeft,
        BandAnswer::Every => crate::domain::growth::service::fan_of(parts.fan),
    }
}

/// The comparison an edge makes, as `right test left`: the one mapping from a band's test to the language's.
#[must_use]
pub(crate) const fn test_of(
    test: ryo_values::domain::plan_parts::types::BandTest,
) -> ryo_values::domain::plan_parts::types::Test {
    use ryo_values::domain::plan_parts::types::{BandTest, Test};
    match test {
        BandTest::Below => Test::LessThan,
        BandTest::AtOrBelow => Test::LessThanOrEqual,
        BandTest::Above => Test::GreaterThan,
        BandTest::AtOrAbove => Test::GreaterThanOrEqual,
    }
}
