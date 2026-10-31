//! Execution timing and binding availability used by lexical resolution.
/// When a lexical scope executes relative to surrounding statements.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScopeExecution {
    /// The scope executes at its source position.
    Immediate,
    /// The body may execute after surrounding declarations are initialized.
    Deferred,
}
/// Whether an immediate assignment has made its target available to this read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BindingAvailability {
    /// The read occurs during evaluation of the binding's initializer.
    Unavailable,
    /// This binding's initialization does not exclude the read.
    Available,
}
