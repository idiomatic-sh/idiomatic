//! The types of the `groups` part: what the operators over the rows of a key: `group-count`, `group-min`, `group-max`, `group-ordinal`, `project` are given. Plain data.

use crate::domain::finish_plans::arguments::types::StepNumber;
use ryo_values::domain::descriptions::types::ColumnName;

/// `group-count`: one row per key, with how many rows (or distinct values of a column) it has.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::finish_plans::arguments::types::StepNumber;
/// use ryo_engine::domain::finish_plans::arguments::groups::types::{CountedRows, GroupCountArguments};
/// use ryo_values::domain::descriptions::types::ColumnName;
///
/// let count = GroupCountArguments { from: StepNumber(0), keys: Vec::new(), count: ColumnName("n".into()), distinct: CountedRows::Every };
/// assert_eq!(count.distinct, CountedRows::Every);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GroupCountArguments {
    /// The rows counted.
    pub from: StepNumber,
    /// The key columns.
    pub keys: Vec<ColumnName>,
    /// The column the count is written to.
    pub count: ColumnName,
    /// What is counted.
    pub distinct: CountedRows,
}

/// What a `group-count` counts: every row, or the distinct values of one column.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::finish_plans::arguments::groups::types::CountedRows;
/// use ryo_values::domain::descriptions::types::ColumnName;
///
/// assert_ne!(CountedRows::Distinct(ColumnName("id".into())), CountedRows::Every);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum CountedRows {
    /// Every row: the query names no `distinct`.
    Every,
    /// The distinct values of this column.
    Distinct(ColumnName),
}

/// `group-min` and `group-max`: one row per key, the one with the least (or the greatest) order.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::finish_plans::arguments::types::StepNumber;
/// use ryo_engine::domain::finish_plans::arguments::groups::types::GroupOrderArguments;
/// use ryo_values::domain::descriptions::types::ColumnName;
///
/// let least = GroupOrderArguments { from: StepNumber(0), keys: Vec::new(), order: ColumnName("line".into()) };
/// assert!(least.keys.is_empty());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GroupOrderArguments {
    /// The rows.
    pub from: StepNumber,
    /// The key columns.
    pub keys: Vec<ColumnName>,
    /// The order column.
    pub order: ColumnName,
}

/// `group-ordinal`: every row with its place among the rows of its key, in the order column's order.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::finish_plans::arguments::types::StepNumber;
/// use ryo_engine::domain::finish_plans::arguments::groups::types::GroupOrdinalArguments;
/// use ryo_values::domain::descriptions::types::ColumnName;
///
/// let name = |text: &str| ColumnName(text.into());
/// let ordinal = GroupOrdinalArguments { from: StepNumber(0), keys: Vec::new(), order: name("line"), output: name("n") };
/// assert_eq!(&*ordinal.output.0, "n");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GroupOrdinalArguments {
    /// The rows.
    pub from: StepNumber,
    /// The key columns.
    pub keys: Vec<ColumnName>,
    /// The order column.
    pub order: ColumnName,
    /// The column the place is written to.
    pub output: ColumnName,
}

/// `project`: the distinct rows of some columns of the input.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::finish_plans::arguments::types::StepNumber;
/// use ryo_engine::domain::finish_plans::arguments::groups::types::ProjectArguments;
///
/// assert!(ProjectArguments { from: StepNumber(0), columns: Vec::new() }.columns.is_empty());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ProjectArguments {
    /// The rows.
    pub from: StepNumber,
    /// The columns kept.
    pub columns: Vec<ColumnName>,
}
