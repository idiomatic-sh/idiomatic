//! The schemas' cases: each of the library's kinds is the kind of the same meaning, and every kind the library names is mapped to a
//! kind of the language or named as dropped with its reason.

use super::super::types::Piece;
use super::piece_kind;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::JsonNode;
use crate::domain::syntax::service::{kind_bits, kind_of};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;

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
        name: CaseName("the_whole_file_is_the_root_node"),
        word: Word::Succeeds,
        input: || Given::One(Piece::File),
        expect: Expect::Value(Seen::Kind(Kind::Node(JsonNode::JsonText))),
    },
    Case {
        name: CaseName("an_object_a_member_and_an_array_are_nodes"),
        word: Word::Succeeds,
        input: || Given::One(Piece::Member),
        expect: Expect::Value(Seen::Kind(Kind::Node(JsonNode::Member))),
    },
    Case {
        name: CaseName("a_brace_is_the_begin_of_an_object"),
        word: Word::Succeeds,
        input: || Given::One(Piece::OpenBrace),
        expect: Expect::Value(Seen::Kind(Kind::Token(Token::BeginObject))),
    },
    Case {
        name: CaseName("a_colon_is_the_name_separator"),
        word: Word::Succeeds,
        input: || Given::One(Piece::Colon),
        expect: Expect::Value(Seen::Kind(Kind::Token(Token::NameSeparator))),
    },
    Case {
        name: CaseName("a_comment_is_an_extra_the_standard_lacks"),
        word: Word::Succeeds,
        input: || Given::One(Piece::LineComment),
        expect: Expect::Value(Seen::Kind(Kind::Extra(Extra::LineComment))),
    },
    Case {
        name: CaseName("a_word_the_grammar_does_not_know_is_text_it_could_not_read"),
        word: Word::Succeeds,
        input: || Given::One(Piece::Word),
        expect: Expect::Value(Seen::Kind(Kind::Extra(Extra::Error))),
    },
    Case {
        name: CaseName("every_kind_of_the_library_is_mapped_or_dropped"),
        word: Word::Succeeds,
        input: || Given::Every,
        expect: Expect::Value(Seen::Accounted(true)),
    },
];

/// Every kind the library names, from the library's own enumeration: one entry for each kind, so that a kind the library adds is
/// tested the day it is added and not when a file first meets it.
fn library_kinds() -> Vec<Piece> {
    vec![
        Piece::File,
        Piece::Object,
        Piece::Member,
        Piece::Array,
        Piece::OpenBrace,
        Piece::CloseBrace,
        Piece::OpenBracket,
        Piece::CloseBracket,
        Piece::Comma,
        Piece::Colon,
        Piece::String,
        Piece::Number,
        Piece::True,
        Piece::False,
        Piece::Null,
        Piece::Word,
        Piece::LineComment,
        Piece::BlockComment,
        Piece::Whitespace,
        Piece::Unread,
    ]
}

/// Why the language has no kind for a kind of the library, or none when it has one. The match names every kind of the library, so
/// that a kind the library adds is a compile error here until it is mapped or named as dropped.
fn dropped(piece: Piece) -> Option<&'static str> {
    match piece {
        Piece::File
        | Piece::Object
        | Piece::Member
        | Piece::Array
        | Piece::OpenBrace
        | Piece::CloseBrace
        | Piece::OpenBracket
        | Piece::CloseBracket
        | Piece::Comma
        | Piece::Colon
        | Piece::String
        | Piece::Number
        | Piece::True
        | Piece::False
        | Piece::Null
        | Piece::LineComment
        | Piece::BlockComment
        | Piece::Whitespace
        | Piece::Word
        | Piece::Unread => None,
    }
}

/// Whether the language answers for `piece`: it is dropped with a reason, or it is mapped to a kind the language has.
fn accounted(piece: Piece) -> bool {
    if let Some(reason) = dropped(piece) {
        !reason.is_empty()
    } else {
        let kind = piece_kind(piece);
        kind_of(kind_bits(kind)) == kind
    }
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
