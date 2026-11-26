//! Retained per-node summaries for the lazy function terminal reader.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Pinned structured terminal state before relation-vocabulary collapsing.
pub(crate) enum TerminalOutcome {
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




/// Whether a `break` can leave the active loop.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Breaks(pub bool);

/// Whether a match pattern is an unguarded wildcard or capture.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct WildcardPattern(pub bool);

/// Whether a terminal summary includes a return path.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Returns(pub bool);
