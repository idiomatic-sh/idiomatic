//! The types of the `replay` concept: what takes the rows of a step as they come.

use ryo_language::domain::relations::types::Value;

use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{Emit, OperatorContext};
use crate::domain::table_model::service::push_row;
use crate::domain::table_model::types::Table;

/// A table that takes the rows as they come.
pub(crate) struct Collect<'t>(pub &'t mut Table);

/// A closure that takes the rows as they come.
pub(crate) struct Pipe<F>(pub F);

impl<F: FnMut(&mut OperatorContext<'_>, &[Value]) -> Result<(), OperatingError>> Emit for Pipe<F> {
    fn row(
        &mut self,
        context: &mut OperatorContext<'_>,
        row: &[Value],
    ) -> Result<(), OperatingError> {
        (self.0)(context, row)
    }
}

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
