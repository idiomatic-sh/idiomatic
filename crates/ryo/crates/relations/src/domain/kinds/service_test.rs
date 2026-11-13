//! The `kinds` service's cases: a kind written from a language's vocabularies is numbered nodes first, then tokens, then extras, read
//! back from its number, named and listed in that order.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::KindBits;

/// The nodes of a language of the cases.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    strum::EnumIter,
    strum::IntoStaticStr,
    strum::FromRepr,
    strum::EnumCount,
)]
#[strum(serialize_all = "kebab-case")]
#[repr(u16)]
enum Node {
    Document,
    KeyValue,
}

/// The tokens of a language of the cases.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    strum::EnumIter,
    strum::IntoStaticStr,
    strum::FromRepr,
    strum::EnumCount,
)]
#[strum(serialize_all = "kebab-case")]
#[repr(u16)]
enum Token {
    Equals,
    Word,
    Space,
}

/// The extras of a language of the cases.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    strum::EnumIter,
    strum::IntoStaticStr,
    strum::FromRepr,
    strum::EnumCount,
)]
#[strum(serialize_all = "kebab-case")]
#[repr(u16)]
enum Extra {
    Error,
}

/// A kind of a language of the cases with extras.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Node(Node),
    Token(Token),
    Extra(Extra),
}

/// A kind of a language of the cases with extras and a further vocabulary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Nested {
    Node(Node),
    Token(Token),
    Extra(Extra),
    Further(Token),
}

/// A kind of a language of the cases without extras.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Plain {
    Node(Node),
    Token(Token),
}

#[expect(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    reason = "a kind is a field-less enum with a u16 representation: its number is the cast, and a count of variants fits a u16"
)]
#[expect(
    clippy::inline_modules,
    reason = "each module scopes one expansion of `kinds!`, whose items would collide in one scope; the cases of the macro are the test's own"
)]
mod with_extras {
    use super::{Extra, Kind, Node, Token};

    crate::kinds!(pub(super) Kind { Node(Node), Token(Token), Extra(Extra) } unknown Kind::Extra(Extra::Error));
}

#[expect(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    reason = "a kind is a field-less enum with a u16 representation: its number is the cast, and a count of variants fits a u16"
)]
#[expect(
    clippy::inline_modules,
    reason = "each module scopes one expansion of `kinds!`, whose items would collide in one scope; the cases of the macro are the test's own"
)]
mod without_extras {
    use super::{Node, Plain, Token};

    crate::kinds!(pub(super) Plain { Node(Node), Token(Token) } unknown Plain::Token(Token::Space));
}

#[expect(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    reason = "a kind is a field-less enum with a u16 representation: its number is the cast, and a count of variants fits a u16"
)]
#[expect(
    clippy::inline_modules,
    reason = "each module scopes one expansion of `kinds!`, whose items would collide in one scope; the cases of the macro are the test's own"
)]
mod with_further {
    use super::{Extra, Nested, Node, Token};

    crate::kinds!(pub(super) Nested { Node(Node), Token(Token), Extra(Extra), Further(Token) } unknown Nested::Extra(Extra::Error));
}

/// A scenario: what a row asks of the numbering, shown as text.
type Input = fn() -> String;
/// What it shows.
type Value = &'static str;
type Failure = ();

/// Every kind with extras as `number:name`, and what the numbers read back as.
fn every_kind_with_extras() -> String {
    with_extras::kinds()
        .map(|kind| {
            let bits = with_extras::kind_bits(kind);
            let back = if with_extras::kind_of(bits) == kind {
                ""
            } else {
                "!"
            };
            format!("{}:{}{back}", bits.0, with_extras::kind_name(kind).0)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Every kind without extras as `number:name`, and what the numbers read back as.
fn every_kind_without_extras() -> String {
    without_extras::kinds()
        .map(|kind| {
            let bits = without_extras::kind_bits(kind);
            let back = if without_extras::kind_of(bits) == kind {
                ""
            } else {
                "!"
            };
            format!("{}:{}{back}", bits.0, without_extras::kind_name(kind).0)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Every kind with a further vocabulary as `number:name`, what the numbers read back as, and what the first number past them reads as.
fn every_kind_with_a_further_vocabulary() -> String {
    let listed = with_further::kinds()
        .map(|kind| {
            let bits = with_further::kind_bits(kind);
            let back = if with_further::kind_of(bits) == kind {
                ""
            } else {
                "!"
            };
            format!("{}:{}{back}", bits.0, with_further::kind_name(kind).0)
        })
        .collect::<Vec<_>>()
        .join(" ");
    format!("{listed} {:?}", with_further::kind_of(KindBits(9)))
}

/// What a number no kind has stands for, with extras and without.
fn a_number_no_kind_has() -> String {
    format!(
        "{:?} {:?}",
        with_extras::kind_of(KindBits(6)),
        without_extras::kind_of(KindBits(5))
    )
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "the_nodes_are_numbered_first_then_the_tokens_then_the_extras_and_each_reads_back",
        ),
        word: Word::Succeeds,
        input: || every_kind_with_extras,
        expect: Expect::Value("0:document 1:key-value 2:equals 3:word 4:space 5:error"),
    },
    Case {
        name: CaseName("a_language_without_extras_numbers_its_nodes_then_its_tokens"),
        word: Word::Succeeds,
        input: || every_kind_without_extras,
        expect: Expect::Value("0:document 1:key-value 2:equals 3:word 4:space"),
    },
    Case {
        name: CaseName("a_further_vocabulary_is_numbered_after_the_extras_and_each_reads_back"),
        word: Word::Succeeds,
        input: || every_kind_with_a_further_vocabulary,
        expect: Expect::Value(
            "0:document 1:key-value 2:equals 3:word 4:space 5:error 6:equals 7:word 8:space Extra(Error)",
        ),
    },
    Case {
        name: CaseName("a_number_no_kind_has_is_the_unknown_kind"),
        word: Word::Succeeds,
        input: || a_number_no_kind_has,
        expect: Expect::Value("Extra(Error) Token(Space)"),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(Box::leak(scenario().into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
