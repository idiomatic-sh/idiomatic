//! The types of the `plan_orders` concept: which row of an ordered key group a step asks for. Plain data, public fields and derives, with no constructor and no inherent impl.

/// Which right rows of a key group an `as-of` step pairs with a left row, by their order against the left row's.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::plan_orders::types::OrderBound;
/// assert_ne!(OrderBound::Before, OrderBound::AtOrBefore);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OrderBound {
    /// The rows of the greatest order strictly below the left row's.
    Before,
    /// The rows of the greatest order at or below the left row's.
    AtOrBefore,
    /// The rows of the least order strictly above the left row's.
    After,
    /// The rows of the least order at or above the left row's.
    AtOrAfter,
}

/// Which end of a key group's order a `group-min` or `group-max` step keeps.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::plan_orders::types::GroupEnd;
/// assert_ne!(GroupEnd::Least, GroupEnd::Greatest);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GroupEnd {
    /// The rows no row of the group is strictly below: `group-min`.
    Least,
    /// The rows no row of the group is strictly above: `group-max`.
    Greatest,
}
