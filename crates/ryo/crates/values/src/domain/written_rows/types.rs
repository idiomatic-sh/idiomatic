//! The types of the `written_rows` concept: the rows a query writes for a `rows` step, and where a `rows` step takes its rows from. Plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::plan_parts::types::{LiteralText, ParameterId};

/// One row a query wrote: its cells, all text, in column order.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::plan_parts::types::LiteralText;
/// use ryo_values::domain::written_rows::types::WrittenRow;
/// let row = WrittenRow(vec![LiteralText("TRIO110".into()), LiteralText("ASYNC110".into())]);
/// assert_eq!(row.0.len(), 2);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WrittenRow(pub Vec<LiteralText>);

/// The rows a query wrote, in the order written; every row is as wide as the step's columns.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::written_rows::types::WrittenRows;
/// assert!(WrittenRows(Vec::new()).0.is_empty());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WrittenRows(pub Vec<WrittenRow>);

/// Where a `rows` step takes its rows from.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::plan_parts::types::ParameterId;
/// use ryo_values::domain::written_rows::types::{RowsSource, WrittenRows};
/// assert_ne!(RowsSource::Parameter(ParameterId(0)), RowsSource::Written(WrittenRows(Vec::new())));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RowsSource {
    /// A `list<table>` parameter the host binds: a row for each table, a column for each key.
    Parameter(ParameterId),
    /// The rows written in the query.
    Written(WrittenRows),
}
