//! The `banding` service: a join's `where` split into its band, the order comparisons between its sides a band join answers,
//! and the rest, which stays the filter after it.

use super::constants::BAND_EDGES;
use super::types::{BandSplit, WrittenEdge};
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::Builder;
use ryo_values::domain::plan_parts::types as parts;

/// A join's `where` split into its band and the rest: each top-level `l.a < r.b` (`<`, `<=`, `>`, `>=`) between a column of
/// each side is an edge, the first two the band, the first of them ordering it; every other condition stays the `where` after
/// the join. A `nearest join` orders by exactly one such comparison.
///
/// # Errors
///
/// `NearestNeedsOneOrder` for a `nearest join` whose `where` compares its sides in order other than once.
pub(crate) fn band_split(
    builder: &Builder<'_>,
    select: &ryo_values::domain::query_syntax::types::Select,
    join: &ryo_values::domain::query_syntax::types::Join,
) -> Result<BandSplit, LoweringError> {
    use ryo_values::domain::query_syntax::types::{Condition, JoinKind};
    let mut split = BandSplit {
        band: Vec::new(),
        rest: Vec::new(),
    };
    let mut ordered = 0_usize;
    for condition in &select.filters {
        let edge = match condition {
            Condition::Test(filter) => edge_of(filter, &select.from.alias, &join.source.alias),
            Condition::Any(_) => None,
        };
        match edge {
            Some(edge) if split.band.len() < BAND_EDGES => {
                ordered += 1;
                split.band.push(edge);
            }
            Some(_) => {
                ordered += 1;
                split.rest.push(condition.clone());
            }
            None => split.rest.push(condition.clone()),
        }
    }
    if join.kind == JoinKind::Nearest && ordered != 1 {
        return Err(LoweringError::NearestNeedsOneOrder {
            count: ordered,
            span: builder.span,
        });
    }
    Ok(split)
}

/// The band edge `filter` is: an order test between a column of the left side and one of the right, as the right one compares
/// with the left one; none for any other test.
fn edge_of(
    filter: &ryo_values::domain::query_syntax::types::Filter,
    left_alias: &ryo_values::domain::query_syntax::types::Alias,
    right_alias: &ryo_values::domain::query_syntax::types::Alias,
) -> Option<WrittenEdge> {
    use ryo_values::domain::band_joins::types::BandTest;
    use ryo_values::domain::query_syntax::types::{Alias, WrittenOperand, WrittenTest};
    let (WrittenOperand::Column(first), WrittenOperand::Column(second)) =
        (&filter.left, &filter.right)
    else {
        return None;
    };
    let side = |reference: &ryo_values::domain::query_syntax::types::ColumnReference| {
        if matches!(reference.alias, Alias::Unwritten) {
            None
        } else if reference.alias == *left_alias {
            Some(parts::Side::Left)
        } else if reference.alias == *right_alias {
            Some(parts::Side::Right)
        } else {
            None
        }
    };
    // `first test second`, read as `right test left`: the right column first, or the test turned around.
    let (left, right, turned) = match (side(first)?, side(second)?) {
        (parts::Side::Left, parts::Side::Right) => (first, second, true),
        (parts::Side::Right, parts::Side::Left) => (second, first, false),
        (parts::Side::Left, parts::Side::Left) | (parts::Side::Right, parts::Side::Right) => {
            return None;
        }
    };
    let test = match (filter.test, turned) {
        (WrittenTest::LessThan, false) | (WrittenTest::GreaterThan, true) => BandTest::Below,
        (WrittenTest::LessThanOrEqual, false) | (WrittenTest::GreaterThanOrEqual, true) => {
            BandTest::AtOrBelow
        }
        (WrittenTest::GreaterThan, false) | (WrittenTest::LessThan, true) => BandTest::Above,
        (WrittenTest::GreaterThanOrEqual, false) | (WrittenTest::LessThanOrEqual, true) => {
            BandTest::AtOrAbove
        }
        (
            WrittenTest::Equal
            | WrittenTest::NotEqual
            | WrittenTest::In
            | WrittenTest::NotIn
            | WrittenTest::GlobAny
            | WrittenTest::NotGlobAny
            | WrittenTest::FloatTextGreaterThanOrEqual
            | WrittenTest::StartsWith
            | WrittenTest::EndsWith
            | WrittenTest::Matches
            | WrittenTest::NumericGreaterThanOrEqual
            | WrittenTest::IntegerSuccessor
            | WrittenTest::NaturalLessThan,
            _,
        ) => return None,
    };
    Some(WrittenEdge {
        left: left.clone(),
        test,
        right: right.clone(),
    })
}
