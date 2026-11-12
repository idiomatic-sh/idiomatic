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
pub(crate) struct TokenLink {
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
    Link(TokenLink),
    Incomplete,
}

/// The unvisited native element positions of one scan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PositionCursor {
    /// The next position to visit.
    pub next: Position,
    /// One past the last position to visit.
    pub end: Position,
}

/// An unmatched opening delimiter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Opener {
    /// The delimiter's token type.
    pub token: Token,
    /// Its place.
    pub at: Position,
}

/// Whether pairing knowledge is whole or was lost.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Pairing {
    /// Every delimiter so far paired.
    Whole,
    /// Pairing knowledge was lost or could not be fully built.
    Lost,
}

/// Whether the incomplete coverage event has been emitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Coverage {
    /// Nothing was emitted yet.
    Pending,
    /// The incomplete coverage event was emitted.
    Reported,
}

/// Invocation-local cursor and delimiter stack; no second token population.
pub(crate) struct Scan<'tree, 'source, 'stack> {
    /// Completed native syntax tree borrowed for one demand.
    pub tree: &'tree Tree<'source>,
    /// Unvisited native element positions, excluding adopted expansions.
    pub positions: PositionCursor,
    /// Previous recognized lexer token for successor edges.
    pub previous: Option<Position>,
    /// Unmatched typed opening delimiters in nesting order.
    pub stack: std::cell::RefMut<'stack, Vec<Opener>>,
    /// At most two remaining edges from the current token.
    pub pending: [Option<TokenLink>; 2],
    /// Pairing knowledge so far.
    pub pairing: Pairing,
    /// The incomplete coverage event's state.
    pub reported: Coverage,
}
