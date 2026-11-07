//! The schemas' cases: each of the library's events, strings and scalars is the kind of the same meaning, and every kind the library
//! names is mapped to a kind of the language or named as dropped with its reason.

use super::super::types::Piece;
use super::piece_kind;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::extras::types::Extra;
use crate::domain::syntax::service::{kind_bits, kind_of};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use toml_parser::decoder::{Encoding, IntegerRadix, ScalarKind};
use toml_parser::parser::EventKind;

/// What the library gave: one kind of it, or all of them.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// One kind of the library.
    One(Piece),
    /// Every kind the library names.
    Every,
}

type Input = Given;
/// The kind of the language it is, or whether every kind of the library is accounted for.
type Value = Seen;
type Failure = ();

/// What a case looks at.
#[derive(Debug, PartialEq, Eq)]
enum Seen {
    /// The kind of the language.
    Kind(Kind),
    /// Whether every kind of the library is mapped to a kind of the language or dropped with a reason.
    Accounted(bool),
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_multi_line_literal_string_is_one"),
        word: Word::Succeeds,
        input: || Given::One(Piece::String(Encoding::MlLiteralString)),
        expect: Expect::Value(Seen::Kind(Kind::Token(Token::MlLiteralString))),
    },
    Case {
        name: CaseName("an_unquoted_string_is_text_the_parser_could_not_read"),
        word: Word::Succeeds,
        input: || Given::One(Piece::Scalar(ScalarKind::String)),
        expect: Expect::Value(Seen::Kind(Kind::Extra(Extra::Error))),
    },
    Case {
        name: CaseName("a_date_is_a_datetime"),
        word: Word::Succeeds,
        input: || Given::One(Piece::Scalar(ScalarKind::DateTime)),
        expect: Expect::Value(Seen::Kind(Kind::Token(Token::DateTime))),
    },
    Case {
        name: CaseName("a_key_value_separator_is_the_equals_sign"),
        word: Word::Succeeds,
        input: || Given::One(Piece::Event(EventKind::KeyValSep)),
        expect: Expect::Value(Seen::Kind(Kind::Token(Token::KeyvalSep))),
    },
    Case {
        name: CaseName("every_kind_of_the_library_is_mapped_or_dropped"),
        word: Word::Succeeds,
        input: || Given::Every,
        expect: Expect::Value(Seen::Accounted(true)),
    },
];

/// Every kind the library names, from the library's own enumerations: its events, the encodings of its strings and the classes of
/// its scalars, one entry for each, so that a kind the library adds is tested the day it is added and not when a file first meets
/// it. (`dropped` names each of them in a match with no wildcard, which stops compiling when one is added.)
fn library_kinds() -> Vec<Piece> {
    let events = [
        EventKind::StdTableOpen,
        EventKind::StdTableClose,
        EventKind::ArrayTableOpen,
        EventKind::ArrayTableClose,
        EventKind::InlineTableOpen,
        EventKind::InlineTableClose,
        EventKind::ArrayOpen,
        EventKind::ArrayClose,
        EventKind::SimpleKey,
        EventKind::KeySep,
        EventKind::KeyValSep,
        EventKind::Scalar,
        EventKind::ValueSep,
        EventKind::Whitespace,
        EventKind::Comment,
        EventKind::Newline,
        EventKind::Error,
    ];
    let strings = [
        Encoding::BasicString,
        Encoding::LiteralString,
        Encoding::MlBasicString,
        Encoding::MlLiteralString,
    ];
    let scalars = [
        ScalarKind::String,
        ScalarKind::Boolean(false),
        ScalarKind::Boolean(true),
        ScalarKind::DateTime,
        ScalarKind::Float,
        ScalarKind::Integer(IntegerRadix::Dec),
        ScalarKind::Integer(IntegerRadix::Hex),
        ScalarKind::Integer(IntegerRadix::Oct),
        ScalarKind::Integer(IntegerRadix::Bin),
    ];
    events
        .into_iter()
        .map(Piece::Event)
        .chain(strings.into_iter().map(Piece::String))
        .chain(scalars.into_iter().map(Piece::Scalar))
        .collect()
}

/// Why the language has no kind for a kind of the library, or none when it has one. The matches name every kind of the library, so
/// that a kind the library adds is a compile error here until it is mapped or named as dropped.
fn dropped(piece: Piece) -> Option<&'static str> {
    match piece {
        Piece::Event(event) => match event {
            EventKind::Scalar => Some(
                "a value is the kind its encoding or its class gives it, which the string and scalar pieces map",
            ),
            EventKind::StdTableOpen
            | EventKind::StdTableClose
            | EventKind::ArrayTableOpen
            | EventKind::ArrayTableClose
            | EventKind::InlineTableOpen
            | EventKind::InlineTableClose
            | EventKind::ArrayOpen
            | EventKind::ArrayClose
            | EventKind::SimpleKey
            | EventKind::KeySep
            | EventKind::KeyValSep
            | EventKind::ValueSep
            | EventKind::Whitespace
            | EventKind::Comment
            | EventKind::Newline
            | EventKind::Error => None,
        },
        Piece::String(encoding) => match encoding {
            Encoding::BasicString
            | Encoding::LiteralString
            | Encoding::MlBasicString
            | Encoding::MlLiteralString => None,
        },
        Piece::Scalar(class) => match class {
            ScalarKind::String
            | ScalarKind::Boolean(_)
            | ScalarKind::DateTime
            | ScalarKind::Float
            | ScalarKind::Integer(_) => None,
        },
    }
}

/// Whether the language answers for `piece`: it is dropped with a reason, or it is mapped to a kind the language has.
fn accounted(piece: Piece) -> bool {
    dropped(piece).map_or_else(
        || {
            let kind = piece_kind(piece);
            kind_of(kind_bits(kind)) == kind
        },
        |reason| !reason.is_empty(),
    )
}

fn subject(given: Input) -> Value {
    match given {
        Given::One(piece) => Seen::Kind(piece_kind(piece)),
        Given::Every => Seen::Accounted(library_kinds().into_iter().all(accounted)),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
