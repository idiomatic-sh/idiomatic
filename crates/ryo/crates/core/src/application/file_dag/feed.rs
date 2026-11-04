//! Application delivery port; semantic items never receive this capability.
use crate::domain::operating::{
    errors::OperatingError,
    types::{Emit, InputPlace, OperatorContext, Supply},
};
use crate::domain::table_model::types::Rows;
use ryo_relations::domain::relations::types::CellKind;
/// What a step reads its inputs through: it can ask for the rows of an input one at a time, or hold them whole.
/// Declared provenance determines whether a missing publication is an error.
#[derive(Clone, Copy, Debug)]
pub(crate) enum InputOrigin {
    External,
    Relation(ryo_relations::domain::relations::types::Relation),
    Internal,
    /// Rows arrive through a declared graph stream with no publication held
    /// at capture time; admission uses physical rows supplied by that
    /// invocation without inventing a retained producer population.
    InternalStream,
}
pub(crate) trait Feed<'f> {
    fn relation_owner(&self) -> std::rc::Rc<super::relations::RelationOwner>;
    fn origin(&self, _which: InputPlace) -> Result<InputOrigin, OperatingError> {
        Ok(InputOrigin::External)
    }
    fn publication(
        &self,
        _which: InputPlace,
    ) -> Result<Option<crate::application::file_dag::relations::PublishedInput>, OperatingError>
    {
        Ok(None)
    }

    /// What the columns of the input hold.
    ///
    /// # Errors
    ///
    /// `NotRead` when the input is a relation nothing provides.
    fn kinds(&self, which: InputPlace) -> Result<&[CellKind], OperatingError>;

    /// Whether the rows of the input are made as they are asked for (`Streamed`), so that they are not at hand to read in place.
    fn streams(&self, which: InputPlace) -> Supply;

    /// Passes the rows of the input to `emit`, one by one, as they are made.
    ///
    /// # Errors
    ///
    /// Whatever the steps that make them or `emit` refuse.
    fn stream(
        &mut self,
        which: InputPlace,
        context: &mut OperatorContext<'_>,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError>;

    /// Notes that a step looked up `rows` rows in the input, which it holds as a tree and does not read whole: the rows it found, and
    /// not the rows the tree has, are what the input was fed.
    fn found(&mut self, which: InputPlace, rows: RowCount);

    /// The rows of the input, whole, to read in place: a relation as it is; the rows of a step, in `into`.
    ///
    /// # Errors
    ///
    /// Whatever the steps that make them refuse.
    fn hold<'t>(
        &mut self,
        which: InputPlace,
        context: &mut OperatorContext<'_>,
        into: &'t mut Table,
    ) -> Result<Rows<'t>, OperatingError>
    where
        'f: 't;
}

use ryo_values::domain::rows::types::{RowCount, Table};
