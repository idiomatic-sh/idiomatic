//! The types of the `replay` concept: what takes the rows of a step as they come.

use ryo_relations::domain::relations::types::Value;

use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{Emit, OperatorContext};
use ryo_values::domain::rows::service::push_row;

/// A table that takes the rows as they come.
pub(crate) struct Collect<'t>(pub &'t mut Table);

impl Emit for Collect<'_> {
    fn row(
        &mut self,
        _context: &mut OperatorContext<'_>,
        row: &[Value],
    ) -> Result<(), OperatingError> {
        push_row(self.0, row);
        Ok(())
    }
}

use ryo_values::domain::rows::types::Table;
