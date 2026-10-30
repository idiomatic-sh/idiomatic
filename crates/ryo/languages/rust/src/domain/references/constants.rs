//! The names of the `references` concept: the text a path is written with.

use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;

/// The separator between the segments of a path.
pub(crate) const SEPARATOR: &str = "::";

/// The name of a `cfg` attribute.
pub(crate) const CFG: &str = "cfg";
/// The name of a `cfg_attr` attribute.
pub(crate) const CFG_ATTR: &str = "cfg_attr";

/// The tokens that open what an attribute passes: `(`, `[` and `{`.
pub(crate) const OPENERS: [Kind; 3] = [
    Kind::Token(Token::LParen),
    Kind::Token(Token::LBrack),
    Kind::Token(Token::LCurly),
];

/// The tokens that close what an attribute passes: `)`, `]` and `}`.
pub(crate) const CLOSERS: [Kind; 3] = [
    Kind::Token(Token::RParen),
    Kind::Token(Token::RBrack),
    Kind::Token(Token::RCurly),
];
