//! Parser-free values retained between literal-expression reads.
use indexmap::IndexMap;
use ryo::languages::Position;

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
    /// Budgeted previous-source grammar results; no source-node identities.
    pub memo: crate::domain::grammar_cache::types::GrammarMemo<LiteralParseOutcome>,
    /// Reusable current-source lookup of already parsed embedded carriers.
    pub embedded: std::collections::HashSet<Position>,
    /// Complete quoted/prefixed token spelling, with token boundaries and dialect preserved.
    pub by_carrier: IndexMap<(LiteralDialect, String), LiteralParseOutcome>,
    /// Current source identities, independent of cache-key sharing.
    pub occurrences: Vec<LiteralParseOccurrence>,
    /// Whether demand has established complete current-source knowledge.
    pub state: LiteralParseState,
}

/// Whether a candidate string carrier belongs to the caller's semantic projection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CarrierVisibility {
    /// The source occurrence participates in the relation.
    Visible,
    /// The source occurrence is outside the selected semantic syntax.
    Hidden,
}

impl From<bool> for CarrierVisibility {
    fn from(visible: bool) -> Self {
        if visible { Self::Visible } else { Self::Hidden }
    }
}
