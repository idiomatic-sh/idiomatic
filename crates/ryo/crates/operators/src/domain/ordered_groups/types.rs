//! What the ordered-group meanings read and answer: one key group's orders, a run of its rows, and a row's place. Plain data, no engine type.

use ryo_relations::domain::relations::types::Integer;

/// One key group's orders, ascending: one for each row of the group, repeats included.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::ordered_groups::types::GroupOrders;
/// use ryo_relations::domain::relations::types::Integer;
/// let orders = [Integer(2), Integer(5)];
/// assert_eq!(GroupOrders(&orders).0.len(), 2);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroupOrders<'a>(pub &'a [Integer]);

/// A run of a group's rows, by their places in its ascending order; empty when no row answers.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::ordered_groups::types::GroupRows;
/// assert!(GroupRows(0..0).0.is_empty());
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupRows(pub std::ops::Range<usize>);

/// A row's place in its group: how many rows of the group have a strictly smaller order.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::ordered_groups::types::Place;
/// assert_eq!(Place(0), Place::default());
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Place(pub u32);
