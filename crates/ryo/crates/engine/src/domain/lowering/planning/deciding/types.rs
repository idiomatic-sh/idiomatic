//! The types of the `deciding` part.

use crate::domain::lowering::flattening::types::RowColumn;
use crate::domain::steps::types::StepOperand;

/// One cell a decision asks of its source: the column's place, and the value asked there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AskedCell {
    /// The column, by place.
    pub(crate) place: RowColumn,
    /// The value.
    pub(crate) value: StepOperand,
}
