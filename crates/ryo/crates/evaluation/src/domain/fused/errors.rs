//! What fusing refuses: the parts of a plan the spike does not evaluate.

/// A plan the fused program cannot take.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum FusedError {
    /// The rule binds parameters: a step shared across rules would read one rule's binding for another.
    #[error("rule {rule} has parameters")]
    Parameters {
        /// The rule, by its place.
        rule: usize,
    },
    /// The rule has steps the spike does not evaluate.
    #[error("rule {rule} has a {operation} step the spike does not evaluate")]
    Unsupported {
        /// The rule, by its place.
        rule: usize,
        /// The operation's name.
        operation: &'static str,
    },
}
