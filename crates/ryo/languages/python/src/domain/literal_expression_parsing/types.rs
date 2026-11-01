//! Parser-free values retained between literal-expression reads.
use ryo::languages::Position;
use std::collections::HashMap;

/// The syntax requested for a reconstructed source carrier.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(crate) enum LiteralDialect {
    /// Ruff's Python string-annotation parser, independent of typing position.
    PythonStringAnnotation,
}

/// A complete syntactic outcome, or explicit unavailable carrier knowledge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LiteralParseOutcome {
    /// The selected grammar accepted the original carrier.
    Parsed,
    /// The carrier was complete but its annotation expression was invalid or empty.
    Malformed,
    /// Logical source or reconstruction was unavailable; never a negative parse result.
    Unavailable,
}

/// Whether demand initialized all current-source carrier occurrences.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum LiteralParseState {
    /// No reader has requested syntactic literal outcomes.
    #[default]
    Fresh,
    /// All eligible occurrences were represented completely.
    Complete,
    /// Some logical source or retained occurrence knowledge was unavailable.
    Partial,
}

/// One existing source identity, with no retained parser AST.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LiteralParseOccurrence {
    /// Native or visible embedded original string expression.
    pub node: Position,
    /// The complete outcome associated with its carrier spelling.
    pub outcome: LiteralParseOutcome,
}

/// An independently demanded per-source cache of positive and negative outcomes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct LiteralParseIndex {
    /// Complete quoted/prefixed token spelling, with token boundaries and dialect preserved.
    pub by_carrier: HashMap<(LiteralDialect, String), LiteralParseOutcome>,
    /// Current source identities, independent of cache-key sharing.
    pub occurrences: Vec<LiteralParseOccurrence>,
    /// Whether demand has established complete current-source knowledge.
    pub state: LiteralParseState,
}
