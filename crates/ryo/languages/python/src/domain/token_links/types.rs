//! Physical native-token link data; no retained parser or cache state.
use crate::domain::tokens::types::Token;
use ryo::languages::{Position};

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

/// An unmatched opening delimiter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Opener {
    /// The delimiter's token type.
    pub(crate) token: Token,
    /// Its place.
    pub(crate) at: Position,
}

/// Whether pairing knowledge is whole or was lost.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Pairing {
    /// Every delimiter so far paired.
    Whole,
    /// Pairing knowledge was lost or could not be fully built.
    Lost,
}
