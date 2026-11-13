//! Why process output cannot be expanded or framed.

/// A refused process output declaration.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ProcessOutputError {
    /// Refuses: the request root, or the path joined below it, is relative, empty or not UTF8.
    #[error("process observation requires an absolute UTF8 request root")]
    RootNotAbsoluteUtf8,
    /// Refuses: an input slot names a file the case does not declare or that escapes the root.
    #[error("process output slot is not a declared safe input")]
    SlotNotDeclared,
}
