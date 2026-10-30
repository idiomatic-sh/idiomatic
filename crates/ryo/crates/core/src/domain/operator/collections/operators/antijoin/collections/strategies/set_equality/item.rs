//! The `set_equality` strategy: a left row stays when the set of the keys of the right does not hold its key, told by the hash of the
//! key and then by the key itself, the right indexed once.

use ryo_language::domain::relations::types::Value;

use super::super::interface::StrategiesItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::collections::operators::antijoin::collections::strategies::types::Absence;
use crate::domain::positions::types::KeyColumns;
use crate::domain::table_model::service::{key_of, matches_empty, matching};
use crate::domain::table_model::types::{Emptiness, KeyIndex, On, Table};

/// `set_equality`, one item of the `strategies` collection.
pub(crate) struct SetEqualityItem;

impl StrategiesItem for SetEqualityItem {
    fn absent(
        &self,
        table: &Table,
        order: &KeyIndex,
        columns: KeyColumns<'_>,
        left_row: &[Value],
        key: &mut Vec<Value>,
    ) -> Result<Absence, OperatingError> {
        let KeyColumns {
            left: left_on,
            right: right_on,
        } = columns;
        key_of(left_row, On::Columns(left_on), key)?;
        Ok(
            if matches_empty(&matching(table, On::Columns(right_on), order, key))
                == Emptiness::Empty
            {
                Absence::Absent
            } else {
                Absence::Present
            },
        )
    }
}
