//! The types of the `operators` part: the arguments of one call, taken one by one as its step is built.

use crate::domain::finish_plans::arguments::literals::types::UnlistedArgument;

/// The arguments of one call not yet taken into its step, in the order written; a name written twice is there once,
/// at its first place, with its last value.
#[derive(Debug)]
pub(crate) struct GivenArguments<'a> {
    /// The operator.
    pub(crate) operator: &'a str,
    /// The arguments left.
    pub(crate) arguments: Vec<UnlistedArgument>,
}
