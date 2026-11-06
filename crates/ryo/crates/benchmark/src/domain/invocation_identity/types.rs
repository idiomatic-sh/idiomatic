//! Data of an invocation spelling.

/// One spelling of an argument, a variable, a path or a boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InvocationText(pub String);

/// The state boundary and the request root a case's invocation is normalized against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Boundaries {
    /// The state boundary of the request.
    pub state: InvocationText,
    /// The request root.
    pub root: InvocationText,
}
