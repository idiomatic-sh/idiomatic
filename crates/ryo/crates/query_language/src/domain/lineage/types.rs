//! The types of the `lineage` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input;

/// A column of a per-file step whose values a column after the files holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ValueOrigin {
    /// The per-file step.
    pub input: Input,
    /// Its column.
    pub column: ColumnIndex,
}
