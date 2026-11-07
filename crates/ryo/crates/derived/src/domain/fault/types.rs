//! The types of the `fault` concept: plain data, public fields and derives, with no constructor and no inherent impl.
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// The place of an input among the inputs a request names, counted from one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputSlot(pub usize);

/// Portable failures; an interrupted engine callback retains its exact error outside this crate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DerivedFault {
    /// The request names an input the caller did not give.
    MissingInput {
        /// The place of the missing input.
        position: InputSlot,
    },
    /// The request names a column the input lacks.
    NoSuchColumn {
        /// The absent column.
        column: ColumnIndex,
    },
    /// Two rows share one key and one order.
    DuplicateOrder,
    /// Two edges share one key and one order.
    DuplicateDfsOrder,
    /// The input is not the shape the request asked for.
    ShapeMismatch,
    /// The operation did not run.
    NotRun,
    /// The inputs are not those of the request.
    WrongOperation,
    /// An engine callback was interrupted.
    Interrupted,
}
