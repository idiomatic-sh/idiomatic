//! The `hash` strategy: the rows of a table are found by the hash of the key of a left row, the table indexed once.

use ryo_language::domain::relations::types::Value;

use super::super::interface::StrategiesItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::positions::types::KeyColumns;
use crate::domain::table_cells::types::RowIndex;
use crate::domain::table_model::service::{key_of, matching};
use crate::domain::table_model::types::{KeyIndex, On, Table};
use crate::domain::tree_join::collections::strategies::types::{Finding, Fitting};

/// `hash`, one item of the `strategies` collection.
pub(crate) struct HashItem;

impl StrategiesItem for HashItem {
    fn applies(&self, finding: Option<Finding>) -> Fitting {
        if finding.is_none() {
            Fitting::Fits
        } else {
            Fitting::Misfits
        }
    }
}

impl HashItem {
    /// The rows of `table`, indexed in `order` by the columns `right_on`, whose key is the key of `left_row` at `left_on`, in the
    /// order they came. `key` is the room the key is made in.
    ///
    /// # Errors
    ///
    /// `NoSuchColumn` when the row lacks a column.
    pub(crate) fn find<'o, 'k>(
        table: &'o Table,
        order: &'o KeyIndex,
        columns: KeyColumns<'k>,
        left_row: &[Value],
        key: &'k mut Vec<Value>,
    ) -> Result<impl Iterator<Item = &'o RowIndex> + Clone + use<'o, 'k>, OperatingError> {
        let KeyColumns {
            left: left_on,
            right: right_on,
        } = columns;
        key_of(left_row, On::Columns(left_on), key)?;
        Ok(matching(table, On::Columns(right_on), order, key))
    }
}
