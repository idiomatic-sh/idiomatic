//! What the parser gives, which the adapter does not control, and its conversion into the language's kinds: the one declaration of
//! how each of the library's events, strings and scalars is a kind of the language's trees.

use super::types::{Piece, Report, ReportMessage};
use crate::domain::extras::types::Extra;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use ryo::languages::Count;
use toml::de::DeTable;
use toml_parser::ParseError;
use toml_parser::decoder::{Encoding, ScalarKind};
use toml_parser::parser::EventKind;

/// How the library names a part of a file, as the language's kind of the same meaning. A key or a value is the kind of its encoding
/// or its class, and a comma the array's or the inline table's by the construct it is in, which only the reader of the events
/// knows; the library's event for each stands at the kind it is when nothing says more.
pub(super) struct PieceSchema(pub Piece);

impl From<PieceSchema> for Kind {
    fn from(schema: PieceSchema) -> Self {
        match schema.0 {
            Piece::Event(event) => match event {
                EventKind::StdTableOpen => Self::Token(Token::StdTableOpen),
                EventKind::StdTableClose => Self::Token(Token::StdTableClose),
                EventKind::ArrayTableOpen => Self::Token(Token::ArrayTableOpen),
                EventKind::ArrayTableClose => Self::Token(Token::ArrayTableClose),
                EventKind::InlineTableOpen => Self::Token(Token::InlineTableOpen),
                EventKind::InlineTableClose => Self::Token(Token::InlineTableClose),
                EventKind::ArrayOpen => Self::Token(Token::ArrayOpen),
                EventKind::ArrayClose => Self::Token(Token::ArrayClose),
                EventKind::SimpleKey => Self::Token(Token::UnquotedKey),
                EventKind::KeySep => Self::Token(Token::DotSep),
                EventKind::KeyValSep => Self::Token(Token::KeyvalSep),
                EventKind::Scalar | EventKind::Error => Self::Extra(Extra::Error),
                EventKind::ValueSep => Self::Token(Token::ArraySep),
                EventKind::Whitespace => Self::Token(Token::Ws),
                EventKind::Comment => Self::Token(Token::Comment),
                EventKind::Newline => Self::Token(Token::Newline),
            },
            Piece::String(encoding) => match encoding {
                Encoding::BasicString => Self::Token(Token::BasicString),
                Encoding::LiteralString => Self::Token(Token::LiteralString),
                Encoding::MlBasicString => Self::Token(Token::MlBasicString),
                Encoding::MlLiteralString => Self::Token(Token::MlLiteralString),
            },
            // An unquoted string is no TOML value: the library reads it only to recover.
            Piece::Scalar(class) => match class {
                ScalarKind::String => Self::Extra(Extra::Error),
                ScalarKind::Boolean(_) => Self::Token(Token::Boolean),
                ScalarKind::DateTime => Self::Token(Token::DateTime),
                ScalarKind::Float => Self::Token(Token::Float),
                ScalarKind::Integer(_) => Self::Token(Token::Integer),
            },
        }
    }
}

/// The language's kind for a part of a file the library names `piece`.
pub(super) fn piece_kind(piece: Piece) -> Kind {
    Kind::from(PieceSchema(piece))
}

/// The language's kind for an event of the library.
pub(super) fn event_kind(event: EventKind) -> Kind {
    piece_kind(Piece::Event(event))
}

/// The language's kind for a string the library encodes as `encoding`.
pub(super) fn string_kind(encoding: Encoding) -> Kind {
    piece_kind(Piece::String(encoding))
}

/// The language's kind for an unquoted scalar the library classified as `kind`.
pub(super) fn scalar_kind(kind: ScalarKind) -> Kind {
    piece_kind(Piece::Scalar(kind))
}

/// What the library says is wrong with the document `text`, once each, in the order it says it: the `toml` crate's recoverable parse,
/// which reports every error of syntax and of meaning; when it finds none, reading the document as values, which says what is wrong
/// with a number; and what the decoder refused of a key or a string (`refusals`).
pub(super) fn reports(text: &str, refusals: &[ParseError]) -> Vec<Report> {
    let (_, mut errors) = DeTable::parse_recoverable(text);
    if errors.is_empty()
        && let Err(error) = toml::from_str::<toml::Table>(text)
    {
        errors.push(error);
    }
    let parsed = errors.iter().map(|error| {
        let span = error.span().unwrap_or_default();
        Report {
            message: ReportMessage(error.message().to_owned()),
            start: Count(span.start),
            end: Count(span.end),
        }
    });
    let decoded = refusals.iter().map(|error| {
        let span = error
            .unexpected()
            .or_else(|| error.context())
            .unwrap_or_default();
        Report {
            message: ReportMessage(error.description().to_owned()),
            start: Count(span.start()),
            end: Count(span.end()),
        }
    });
    let mut said: Vec<Report> = Vec::new();
    for each in parsed.chain(decoded) {
        if !said.contains(&each) {
            said.push(each);
        }
    }
    said
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
