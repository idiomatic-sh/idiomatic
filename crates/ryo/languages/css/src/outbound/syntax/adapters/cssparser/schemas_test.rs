//! The schemas' cases: each of the library's tokens is the kind of the same meaning, each error says what the library says, and every
//! kind the library names is mapped to a kind of the language or dropped with its reason.

use super::{error_message, met, token_kind};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::syntax::service::{kind_bits, kind_of};
use crate::domain::syntax::types::Kind;
use crate::outbound::syntax::adapters::cssparser::types::{Met, Scratch};
use cssparser::{BasicParseErrorKind, CowRcStr, ParseErrorKind, Token};
use strum::IntoEnumIterator as _;

/// A kind of the library: one for each variant of its `Token`, which `piece_of` names exhaustively.
#[derive(Clone, Copy, Debug, PartialEq, Eq, strum::EnumIter)]
enum Piece {
    Ident,
    AtKeyword,
    Hash,
    IdHash,
    QuotedString,
    UnquotedUrl,
    Delim,
    Number,
    Percentage,
    Dimension,
    WhiteSpace,
    Comment,
    Colon,
    Semicolon,
    Comma,
    IncludeMatch,
    DashMatch,
    PrefixMatch,
    SuffixMatch,
    SubstringMatch,
    Cdo,
    Cdc,
    Function,
    ParenthesisBlock,
    SquareBracketBlock,
    CurlyBracketBlock,
    BadUrl,
    BadString,
    CloseParenthesis,
    CloseSquareBracket,
    CloseCurlyBracket,
}

/// What the library gave.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// One kind of it, a token.
    One(Piece),
    /// An error of its rule parsers: an invalid qualified rule.
    InvalidRule,
    /// Every kind the library names.
    Every,
}

type Input = Given;
/// What the adapter makes of it.
type Value = Seen;
type Failure = ();

/// What a case looks at.
#[derive(Debug, PartialEq, Eq)]
enum Seen {
    /// The kind of the language, and what the adapter makes of the token walking a run.
    Kind(Kind, Met),
    /// What the error says.
    Message(&'static str),
    /// Whether every kind of the library is mapped to a kind of the language or dropped with a reason.
    Accounted(bool),
}

/// The kind of the library a token is. The match names every variant of the library's `Token`, so that a token it adds is a compile
/// error here until it is a kind of the list.
fn piece_of(token: &Token<'_>) -> Piece {
    match token {
        Token::Ident(_) => Piece::Ident,
        Token::AtKeyword(_) => Piece::AtKeyword,
        Token::Hash(_) => Piece::Hash,
        Token::IDHash(_) => Piece::IdHash,
        Token::QuotedString(_) => Piece::QuotedString,
        Token::UnquotedUrl(_) => Piece::UnquotedUrl,
        Token::Delim(_) => Piece::Delim,
        Token::Number { .. } => Piece::Number,
        Token::Percentage { .. } => Piece::Percentage,
        Token::Dimension { .. } => Piece::Dimension,
        Token::WhiteSpace(_) => Piece::WhiteSpace,
        Token::Comment(_) => Piece::Comment,
        Token::Colon => Piece::Colon,
        Token::Semicolon => Piece::Semicolon,
        Token::Comma => Piece::Comma,
        Token::IncludeMatch => Piece::IncludeMatch,
        Token::DashMatch => Piece::DashMatch,
        Token::PrefixMatch => Piece::PrefixMatch,
        Token::SuffixMatch => Piece::SuffixMatch,
        Token::SubstringMatch => Piece::SubstringMatch,
        Token::CDO => Piece::Cdo,
        Token::CDC => Piece::Cdc,
        Token::Function(_) => Piece::Function,
        Token::ParenthesisBlock => Piece::ParenthesisBlock,
        Token::SquareBracketBlock => Piece::SquareBracketBlock,
        Token::CurlyBracketBlock => Piece::CurlyBracketBlock,
        Token::BadUrl(_) => Piece::BadUrl,
        Token::BadString(_) => Piece::BadString,
        Token::CloseParenthesis => Piece::CloseParenthesis,
        Token::CloseSquareBracket => Piece::CloseSquareBracket,
        Token::CloseCurlyBracket => Piece::CloseCurlyBracket,
    }
}

/// A token of the library of the kind `piece`.
fn sample(piece: Piece) -> Token<'static> {
    let text = || CowRcStr::from("a");
    match piece {
        Piece::Ident => Token::Ident(text()),
        Piece::AtKeyword => Token::AtKeyword(text()),
        Piece::Hash => Token::Hash(text()),
        Piece::IdHash => Token::IDHash(text()),
        Piece::QuotedString => Token::QuotedString(text()),
        Piece::UnquotedUrl => Token::UnquotedUrl(text()),
        Piece::Delim => Token::Delim('+'),
        Piece::Number => Token::Number {
            has_sign: false,
            value: 1.0,
            int_value: Some(1),
        },
        Piece::Percentage => Token::Percentage {
            has_sign: false,
            unit_value: 0.5,
            int_value: Some(50),
        },
        Piece::Dimension => Token::Dimension {
            has_sign: false,
            value: 1.0,
            int_value: Some(1),
            unit: text(),
        },
        Piece::WhiteSpace => Token::WhiteSpace(" "),
        Piece::Comment => Token::Comment(" a "),
        Piece::Colon => Token::Colon,
        Piece::Semicolon => Token::Semicolon,
        Piece::Comma => Token::Comma,
        Piece::IncludeMatch => Token::IncludeMatch,
        Piece::DashMatch => Token::DashMatch,
        Piece::PrefixMatch => Token::PrefixMatch,
        Piece::SuffixMatch => Token::SuffixMatch,
        Piece::SubstringMatch => Token::SubstringMatch,
        Piece::Cdo => Token::CDO,
        Piece::Cdc => Token::CDC,
        Piece::Function => Token::Function(text()),
        Piece::ParenthesisBlock => Token::ParenthesisBlock,
        Piece::SquareBracketBlock => Token::SquareBracketBlock,
        Piece::CurlyBracketBlock => Token::CurlyBracketBlock,
        Piece::BadUrl => Token::BadUrl(text()),
        Piece::BadString => Token::BadString(text()),
        Piece::CloseParenthesis => Token::CloseParenthesis,
        Piece::CloseSquareBracket => Token::CloseSquareBracket,
        Piece::CloseCurlyBracket => Token::CloseCurlyBracket,
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_hash_of_either_type_is_a_hash_token"),
        word: Word::Succeeds,
        input: || Given::One(Piece::IdHash),
        expect: Expect::Value(Seen::Kind(
            Kind::Token(crate::domain::tokens::types::Token::Hash),
            Met::Content,
        )),
    },
    Case {
        name: CaseName("a_function_token_opens_a_function"),
        word: Word::Succeeds,
        input: || Given::One(Piece::Function),
        expect: Expect::Value(Seen::Kind(
            Kind::Token(crate::domain::tokens::types::Token::Function),
            Met::Opens(crate::domain::nodes::types::Node::Function),
        )),
    },
    Case {
        name: CaseName("a_comment_is_an_extra_and_trivia"),
        word: Word::Succeeds,
        input: || Given::One(Piece::Comment),
        expect: Expect::Value(Seen::Kind(
            Kind::Extra(crate::domain::extras::types::Extra::Comment),
            Met::Trivia,
        )),
    },
    Case {
        name: CaseName("an_attribute_matcher_is_delim_tokens"),
        word: Word::Succeeds,
        input: || Given::One(Piece::IncludeMatch),
        expect: Expect::Value(Seen::Kind(
            Kind::Token(crate::domain::tokens::types::Token::Delim),
            Met::Content,
        )),
    },
    Case {
        name: CaseName("an_invalid_rule_says_what_the_library_says"),
        word: Word::Succeeds,
        input: || Given::InvalidRule,
        expect: Expect::Value(Seen::Message("invalid qualified rule encountered")),
    },
    Case {
        name: CaseName("every_kind_of_the_library_is_mapped_or_dropped"),
        word: Word::Succeeds,
        input: || Given::Every,
        expect: Expect::Value(Seen::Accounted(true)),
    },
];

/// Every kind the library names, one for each variant of its `Token`, which `Piece` lists and `piece_of` holds to the library's own
/// enumeration.
fn library_kinds() -> Vec<Piece> {
    Piece::iter().collect()
}

/// Why the language has no kind for a kind of the library, or none when it has one. The match names every kind, so that a kind the
/// library adds is a compile error here until it is mapped or named as dropped. Each token of the library is a token of the
/// standard, two of them, or the extra for a comment, so none is dropped.
fn dropped(piece: Piece) -> Option<&'static str> {
    match piece {
        Piece::Ident
        | Piece::AtKeyword
        | Piece::Hash
        | Piece::IdHash
        | Piece::QuotedString
        | Piece::UnquotedUrl
        | Piece::Delim
        | Piece::Number
        | Piece::Percentage
        | Piece::Dimension
        | Piece::WhiteSpace
        | Piece::Comment
        | Piece::Colon
        | Piece::Semicolon
        | Piece::Comma
        | Piece::IncludeMatch
        | Piece::DashMatch
        | Piece::PrefixMatch
        | Piece::SuffixMatch
        | Piece::SubstringMatch
        | Piece::Cdo
        | Piece::Cdc
        | Piece::Function
        | Piece::ParenthesisBlock
        | Piece::SquareBracketBlock
        | Piece::CurlyBracketBlock
        | Piece::BadUrl
        | Piece::BadString
        | Piece::CloseParenthesis
        | Piece::CloseSquareBracket
        | Piece::CloseCurlyBracket => None,
    }
}

/// Whether the language answers for `piece`: it is dropped with a reason, or it is mapped to a kind the language has, from a token
/// of the library that is of that kind.
fn accounted(piece: Piece) -> bool {
    dropped(piece).map_or_else(
        || {
            let token = sample(piece);
            let kind = token_kind(&token);
            piece_of(&token) == piece && kind_of(kind_bits(kind)) == kind
        },
        |reason| !reason.is_empty(),
    )
}

fn subject(given: Input) -> Value {
    match given {
        Given::One(piece) => {
            let token = sample(piece);
            Seen::Kind(token_kind(&token), met(&token))
        }
        Given::InvalidRule => {
            let kind = ParseErrorKind::<()>::Basic(BasicParseErrorKind::QualifiedRuleInvalid);
            let mut text = Scratch::default();
            error_message(&kind, &mut text);
            Seen::Message(Box::leak(text.0.into_boxed_str()))
        }
        Given::Every => Seen::Accounted(library_kinds().into_iter().all(accounted)),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| {
        Ok::<Value, Failure>(subject(input))
    });
}
