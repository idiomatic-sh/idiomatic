//! Why an invocation cannot be normalized.

/// A refused invocation.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum InvocationIdentityError {
    /// Refuses: an argument, variable or directory is not UTF8.
    #[error("non-UTF8 reference invocation")]
    NonUtf8,
}
