//! The test data of `linter lint` that fixes the refusals its rows tell apart.

/// The refusals the rows tell apart.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// Preserve the engine execution reason for an unexpected runtime refusal.
    Execution(String),
    /// Invalid declared observation bindings.
    ObservationBindings,
    /// Required observation publication failed during execution.
    ObservationExecution,
    /// The explicitly requested source provider is not linked.
    SourceLanguage,
    /// A rule whose text is wrong.
    Rule,
    /// A rule that cannot be run.
    Unrunnable,
    /// A parameter nothing is bound to.
    MissingParameter,
    /// A repository that does not fit.
    TooLarge,
    /// A path that is not text.
    PathNotText,
    /// A part of the system failed.
    Io,
    /// A language wrote what its file cannot hold.
    Provider,
    /// The host declared its relations wrongly.
    Host,
    /// The run held more than the budget.
    MemoryBudgetExceeded,
}
