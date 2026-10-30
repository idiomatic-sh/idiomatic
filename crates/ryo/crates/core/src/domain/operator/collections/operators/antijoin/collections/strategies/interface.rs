//! The shape every item of the `strategies` collection implements.

use ryo_language::domain::relations::types::Value;

use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::collections::operators::antijoin::collections::strategies::types::Absence;
use crate::domain::positions::types::KeyColumns;
use crate::domain::table_model::types::{KeyIndex, Table};

/// What every item of the `strategies` collection offers: whether a row on the left of an antijoin has no match on the right.
pub(crate) trait StrategiesItem {
    /// Whether no row of `table`, indexed in `order` by the columns `right_on`, has the key `left_row` has at `left_on`. `key` is
    /// the room the key is made in.
    ///
    /// # Errors
    ///
    /// `NoSuchColumn` when the row lacks a column.
    fn absent(
        &self,
        table: &Table,
        order: &KeyIndex,
        columns: KeyColumns<'_>,
        left_row: &[Value],
        key: &mut Vec<Value>,
    ) -> Result<Absence, OperatingError>;
}
