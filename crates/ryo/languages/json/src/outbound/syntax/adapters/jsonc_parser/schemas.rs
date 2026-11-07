//! What the parser gives, which the adapter does not control, and its conversion into the language's kinds: the one declaration of
//! how each of the library's kinds is a kind of the language's trees.

use jsonc_parser::tokens::Token as Scanned;

use super::types::Piece;
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;

/// How the library names a part of a file.
pub(super) struct PieceSchema(pub Piece);

impl From<PieceSchema> for Kind {
    fn from(schema: PieceSchema) -> Self {
        match schema.0 {
            Piece::File => Self::Node(Node::JsonText),
            Piece::Object => Self::Node(Node::Object),
            Piece::Member => Self::Node(Node::Member),
            Piece::Array => Self::Node(Node::Array),
            Piece::OpenBrace => Self::Token(Token::BeginObject),
            Piece::CloseBrace => Self::Token(Token::EndObject),
            Piece::OpenBracket => Self::Token(Token::BeginArray),
            Piece::CloseBracket => Self::Token(Token::EndArray),
            Piece::Comma => Self::Token(Token::ValueSeparator),
            Piece::Colon => Self::Token(Token::NameSeparator),
            Piece::String => Self::Token(Token::String),
            Piece::Number => Self::Token(Token::Number),
            Piece::True => Self::Token(Token::True),
            Piece::False => Self::Token(Token::False),
            Piece::Null => Self::Token(Token::Null),
            Piece::Whitespace => Self::Token(Token::Ws),
            Piece::LineComment => Self::Extra(Extra::LineComment),
            Piece::BlockComment => Self::Extra(Extra::BlockComment),
            Piece::Word | Piece::Unread => Self::Extra(Extra::Error),
        }
    }
}

/// The language's kind for a part of a file the library names `piece`.
pub(super) fn piece_kind(piece: Piece) -> Kind {
    Kind::from(PieceSchema(piece))
}

/// What the library calls a token it scanned.
pub(super) fn token_piece(token: &Scanned<'_>) -> Piece {
    match token {
        Scanned::OpenBrace => Piece::OpenBrace,
        Scanned::CloseBrace => Piece::CloseBrace,
        Scanned::OpenBracket => Piece::OpenBracket,
        Scanned::CloseBracket => Piece::CloseBracket,
        Scanned::Comma => Piece::Comma,
        Scanned::Colon => Piece::Colon,
        Scanned::String(_) => Piece::String,
        Scanned::Word(_) => Piece::Word,
        Scanned::Boolean(true) => Piece::True,
        Scanned::Boolean(false) => Piece::False,
        Scanned::Number(_) => Piece::Number,
        Scanned::Null => Piece::Null,
        Scanned::CommentLine(_) => Piece::LineComment,
        Scanned::CommentBlock(_) => Piece::BlockComment,
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
