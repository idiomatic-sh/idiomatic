//! The schemas' cases: each event of the parser becomes the adapter's own step, and every kind the library names is mapped to a kind of
//! the language or named as dropped with its reason.

use ra_ap_parser::StrStep;
use ra_ap_syntax::SyntaxKind;

use super::super::types::Step;
use super::{kind_of as kind_of_library, step_of};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::extras::types::Extra;
use crate::domain::syntax::service::{kind_bits, kind_name, kind_of};
use crate::domain::syntax::types::Kind;

/// Which event of the parser is converted.
#[derive(Clone, Copy, Debug)]
enum Event {
    /// A node is entered.
    Entered,
    /// A token of five bytes.
    Token,
    /// A node is left.
    Left,
    /// The parser could not read the text at byte three.
    Error,
}

/// What the library gave: one event of its parser, or all of its kinds.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// One event of the parser.
    One(Event),
    /// Every kind the library names.
    Every,
}

type Input = Given;
/// What a case looks at.
#[derive(Debug, PartialEq, Eq)]
enum Seen {
    /// Which step it became, and the number it carries: the bytes of a token, the offset of an error.
    Step(u32, u32),
    /// Whether every kind of the library is mapped to a kind of the language or dropped with a reason.
    Accounted(bool),
}

type Value = Seen;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_entered_node_is_an_enter_step"),
        word: Word::Succeeds,
        input: || Given::One(Event::Entered),
        expect: Expect::Value(Seen::Step(0, 0)),
    },
    Case {
        name: CaseName("a_token_keeps_how_many_bytes_it_has"),
        word: Word::Succeeds,
        input: || Given::One(Event::Token),
        expect: Expect::Value(Seen::Step(1, 5)),
    },
    Case {
        name: CaseName("a_left_node_is_an_exit_step"),
        word: Word::Succeeds,
        input: || Given::One(Event::Left),
        expect: Expect::Value(Seen::Step(2, 0)),
    },
    Case {
        name: CaseName("an_error_keeps_where_the_parser_stopped"),
        word: Word::Succeeds,
        input: || Given::One(Event::Error),
        expect: Expect::Value(Seen::Step(3, 3)),
    },
    Case {
        name: CaseName("every_kind_of_the_library_is_mapped_or_dropped"),
        word: Word::Succeeds,
        input: || Given::Every,
        expect: Expect::Value(Seen::Accounted(true)),
    },
];

/// Every kind the library names, from the library's own enumeration: its kinds are numbered from zero to `__LAST`, so that a kind the
/// library adds is tested the day it is added and not when a file first meets it.
fn library_kinds() -> Vec<SyntaxKind> {
    (0..=u16::from(SyntaxKind::__LAST))
        .map(SyntaxKind::from)
        .collect()
}

/// Why the language has no kind for a kind of the library, or none when it has one. The match names every kind of the language, so
/// that a kind the language gains is a compile error here until it is said what it is; the library's kinds are named without a
/// wildcard by `kind_of`, so that a kind the library gains fails to compile there until it is mapped or dropped.
fn dropped(kind: SyntaxKind) -> Option<&'static str> {
    match kind_of_library(kind) {
        Kind::Extra(Extra::Other) => Some(
            "the parser's three markers, TOMBSTONE, EOF and __LAST, which no tree it builds holds",
        ),
        Kind::Node(_) | Kind::Token(_) => None,
    }
}

/// Whether the language answers for `kind`: it is dropped with a reason, or it is mapped to the kind of its own name.
fn accounted(kind: SyntaxKind) -> bool {
    if let Some(reason) = dropped(kind) {
        return !reason.is_empty()
            && matches!(
                kind,
                SyntaxKind::TOMBSTONE | SyntaxKind::EOF | SyntaxKind::__LAST
            );
    }
    let ours = kind_of_library(kind);
    kind_of(kind_bits(ours)) == ours
        && kind_name(ours).0 == format!("{kind:?}").to_lowercase().replace('_', "-")
}

fn subject(given: Input) -> Value {
    let Given::One(event) = given else {
        return Seen::Accounted(library_kinds().into_iter().all(accounted));
    };
    let step = match event {
        Event::Entered => step_of(StrStep::Enter {
            kind: SyntaxKind::SOURCE_FILE,
        }),
        Event::Token => step_of(StrStep::Token {
            kind: SyntaxKind::IDENT,
            text: "hello",
        }),
        Event::Left => step_of(StrStep::Exit),
        Event::Error => step_of(StrStep::Error { msg: "bad", pos: 3 }),
    };
    match step {
        Step::Enter(_) => Seen::Step(0, 0),
        Step::Token(_, count) => Seen::Step(1, u32::try_from(count.0).unwrap_or(u32::MAX)),
        Step::Exit => Seen::Step(2, 0),
        Step::Error(_, offset) => Seen::Step(3, offset.0),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
