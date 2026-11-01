//! Retained per-node summaries for the lazy function terminal reader.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Pinned structured terminal state before relation-vocabulary collapsing.
pub(crate) enum Outcome {
    /// A body falls through without a recognized terminal statement.
    Implicit,
    /// Every represented path raises.
    Raise,
    /// Every represented path returns.
    Return,
    /// Every represented path returns or raises.
    RaiseOrReturn,
    /// At least one path returns while another can fall through.
    ConditionalReturn,
    /// The fold has not encountered a terminal effect yet.
    #[default]
    None,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Bottom-up analysis retained for one syntax node.
pub(crate) struct NodeSummary {
    /// Terminal effect when the node is a statement.
    pub terminal: Outcome,
    /// Terminal result when the node defines a function.
    pub function_terminal: Outcome,
    /// Whether a `break` can leave the active loop through this statement.
    pub sometimes_breaks: bool,
    /// Whether a match pattern is an unguarded wildcard or capture.
    pub wildcard_pattern: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Whether the source-local terminal index was requested and fully represented.
pub(crate) enum IndexState {
    #[default]
    /// No relation reader has requested a build since the last parse.
    Fresh,
    /// The current tree was indexed completely.
    Complete,
    /// The tree or retained storage was incomplete.
    Partial,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
/// Capacity-retaining summaries for one parsed source.
pub(crate) struct TerminalIndex {
    /// One summary per element table position.
    pub nodes: Vec<NodeSummary>,
    /// Validity of the retained summaries.
    pub state: IndexState,
}
