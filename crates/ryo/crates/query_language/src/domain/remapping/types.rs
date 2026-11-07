//! The types of the `remapping` concept: plain data, public fields and derives.



/// Whether a call's parameters must all be declared by the fragment that makes the call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ParameterCheck {
    /// An undeclared parameter is refused.
    Strict,
    /// An undeclared parameter is left for the caller's own scope to answer.
    Lenient,
}
