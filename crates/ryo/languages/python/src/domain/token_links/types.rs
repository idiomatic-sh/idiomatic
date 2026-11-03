//! Physical native-token link data; no retained parser or cache state.
use crate::domain::tokens::types::Token;
use ryo::languages::{Position, Tree};

/// Three independent structural edge families.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LinkKind {
    /// Consecutive physical lexer tokens, whitespace gaps excluded.
    Next,
    /// A type-matching opening and closing delimiter.
    Matching,
    /// Token to its immediately enclosing opener; closing tokens exclude their own pair.
    Enclosing,
}

/// One edge between existing native token identities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Link {
    /// Existing source token identity.
    pub source: Position,
    /// Existing target token identity.
    pub target: Position,
    /// Structural edge vocabulary.
    pub kind: LinkKind,
}

/// A streaming result or explicit incomplete pairing knowledge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Event {
    Link(Link),
    Incomplete,
}

/// Invocation-local cursor and delimiter stack; no second token population.
pub(crate) struct Scan<'tree, 'source> {
    /// Completed native syntax tree borrowed for one demand.
    pub tree: &'tree Tree<'source>,
    /// Unvisited native element positions, excluding adopted expansions.
    pub positions: std::ops::Range<u32>,
    /// Previous recognized lexer token for successor edges.
    pub previous: Option<Position>,
    /// Unmatched typed opening delimiters in nesting order.
    pub stack: Vec<(Token, Position)>,
    /// At most two remaining edges from the current token.
    pub pending: [Option<Link>; 2],
    /// Pairing knowledge was lost or could not be fully built.
    pub incomplete: bool,
    /// The incomplete coverage event has already been emitted.
    pub reported: bool,
}
