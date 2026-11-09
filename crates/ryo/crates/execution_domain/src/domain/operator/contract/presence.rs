//! What an operation declares of the rows it needs and the rows that can make it fail, read by the presence fold.

/// Which inputs must have a row in a file for an operation to answer any row there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresenceMask {
    /// Every input the mask names must have a row.
    AllOf(InputMask),
    /// At least one input the mask names must have a row.
    AnyOf(InputMask),
    /// The answer can have rows whatever the inputs hold.
    Always,
}

/// Inputs by their place: the bit of an input is `1 << place`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputMask(pub u8);

/// Which inputs' rows can make an operation fail on what the rows hold, not on their schema.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowFailure {
    /// No row of any input can make it fail.
    Never,
    /// A row of an input the mask names can make it fail.
    OnRows(InputMask),
}
