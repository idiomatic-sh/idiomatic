//! The types of the `presence` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// Which inputs must have a row in a file for an operation to answer any row there.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::operator::contract::presence::types::PresenceMask;
/// fn named(_: Option<&PresenceMask>) {}
/// ```
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
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::operator::contract::presence::types::InputMask;
/// fn named(_: Option<&InputMask>) {}
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputMask(pub u8);

/// Which inputs' rows can make an operation fail on what the rows hold, not on their schema.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::operator::contract::presence::types::RowFailure;
/// fn named(_: Option<&RowFailure>) {}
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowFailure {
    /// No row of any input can make it fail.
    Never,
    /// A row of an input the mask names can make it fail.
    OnRows(InputMask),
    /// A row of the first input can make it fail unless the span column it searches is located, a span the file's
    /// provider admitted or a match inside one, and its literal pattern compiled with its capture when the request was
    /// prepared. The presence fold decides which.
    UnlessLocated(ryo_values::domain::plan_parts::types::ColumnIndex),
}
