//! The shape every item of the `operators` collection implements.

use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{Emit, Feed, OperatorContext, Room};
use crate::domain::table_model::types::{Rows, Table};
use ryo_values::domain::plans::types::Operation;

/// What every item of the `operators` collection offers: the rows it
/// computes from the rows of its inputs into the table it is given, whose room it reuses.
pub(crate) trait OperatorsItem: Sync {
    /// Computes the rows of one step. The operation is the item's own; any other is refused.
    ///
    /// # Errors
    ///
    /// An `OperatingError` naming what is missing or wrong.
    fn compute(
        &self,
        operation: &Operation,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError>;

    /// Passes on the rows of one step as they are made. An item that cannot pass a row on when it has read the row it comes
    /// from holds the inputs whole, computes the rows and passes them on (`replay_computed`).
    ///
    /// # Errors
    ///
    /// Whatever `compute` refuses, or the inputs or the one that takes the rows refuse.
    fn stream(
        &self,
        operation: &Operation,
        ins: &mut dyn Feed<'_>,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError>;
}
