//! What every `syntax` adapter must pass: the elements it makes of a source, compared in full with elements written by hand.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::constants::EXPANDED_MACROS;
use crate::domain::syntax::service::kind_of;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use crate::outbound::syntax::types::{Expanding, SourceText};
use ryo::languages::{
    ByteSlice, ElementOrigin, Elements, Position, Symbols, decoded_at, end_at, end_of, kind_at,
    origin_at, start_at, table_builder,
};

use super::super::interface::SyntaxPort;

/// An element of the table: its kind, and the text it spans.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Want {
    /// Its kind.
    pub kind: Kind,
    /// The text it spans.
    pub text: &'static str,
}

/// An element of the table: where it comes from, its kind, and the text it spans.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Made {
    /// Where it comes from.
    pub origin: Origin,
    /// Its kind and the text it spans.
    pub element: Want,
    /// The text the adapter's library decoded it to stand for, when it did: the value of a string.
    pub value: Decoding,
}

/// Where an element of the table comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Origin {
    /// A parser-created node without source bytes.
    Implied,
    /// The file wrote it.
    File,
    /// The adapter read the arguments of the macro call of this kind again, and this is one of the elements it found.
    Expansion(Kind),
}

/// What the adapter's library decoded an element to stand for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Decoding {
    /// The element stands for the text it spells.
    Plain,
    /// The library decoded it to this text: the value of a string.
    Decoded(&'static str),
}

/// Which macros the adapter is told to read again.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Told {
    /// The standard macros whose arguments are expressions.
    Standard,
    /// None.
    Nothing,
}

/// What a row of the contract is given: Rust source text, and which macros are read again.
type Input = (&'static str, Told);
/// Every element of the table, in order, with where it comes from.
type Value = &'static [Made];
/// A syntax port refuses nothing: the parser recovers from what it cannot read.
type Failure = ();

/// An element written in the file itself, with no decoded value.
const fn own(kind: Kind, text: &'static str) -> Made {
    Made {
        origin: Origin::File,
        element: Want { kind, text },
        value: Decoding::Plain,
    }
}

/// An element a macro call expanded to, with no decoded value.
const fn expanded(kind: Kind, text: &'static str) -> Made {
    Made {
        origin: Origin::Expansion(Kind::Node(Node::MacroCall)),
        element: Want { kind, text },
        value: Decoding::Plain,
    }
}

/// A string token whose value the library decoded.
const fn string(text: &'static str, value: &'static str, origin: Origin) -> Made {
    Made {
        origin,
        element: Want {
            kind: Kind::Token(Token::String),
            text,
        },
        value: Decoding::Decoded(value),
    }
}

/// The contract's rows: a name, one case word, an input and the elements expected. Every adapter runs them.
pub(crate) const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_function_gives_its_elements_in_order"),
        word: Word::Succeeds,
        input: || ("fn main() {}", Told::Standard),
        expect: Expect::Value(&[
            own(Kind::Node(Node::SourceFile), "fn main() {}"),
            own(Kind::Node(Node::Fn), "fn main() {}"),
            own(Kind::Token(Token::FnKw), "fn"),
            own(Kind::Token(Token::Whitespace), " "),
            own(Kind::Node(Node::Name), "main"),
            own(Kind::Token(Token::Ident), "main"),
            own(Kind::Node(Node::ParamList), "()"),
            own(Kind::Token(Token::LParen), "("),
            own(Kind::Token(Token::RParen), ")"),
            own(Kind::Token(Token::Whitespace), " "),
            own(Kind::Node(Node::BlockExpr), "{}"),
            own(Kind::Node(Node::StmtList), "{}"),
            own(Kind::Token(Token::LCurly), "{"),
            own(Kind::Token(Token::RCurly), "}"),
        ]),
    },
    Case {
        name: CaseName("an_empty_file_gives_only_its_root"),
        word: Word::Succeeds,
        input: || ("", Told::Standard),
        expect: Expect::Value(&[own(Kind::Node(Node::SourceFile), "")]),
    },
    Case {
        name: CaseName("the_arguments_of_a_standard_macro_are_added_under_its_call"),
        word: Word::Succeeds,
        input: || ("fn f() { println!(\"{}\", a); }", Told::Standard),
        expect: Expect::Value(&[
            own(
                Kind::Node(Node::SourceFile),
                "fn f() { println!(\"{}\", a); }",
            ),
            own(Kind::Node(Node::Fn), "fn f() { println!(\"{}\", a); }"),
            own(Kind::Token(Token::FnKw), "fn"),
            own(Kind::Token(Token::Whitespace), " "),
            own(Kind::Node(Node::Name), "f"),
            own(Kind::Token(Token::Ident), "f"),
            own(Kind::Node(Node::ParamList), "()"),
            own(Kind::Token(Token::LParen), "("),
            own(Kind::Token(Token::RParen), ")"),
            own(Kind::Token(Token::Whitespace), " "),
            own(Kind::Node(Node::BlockExpr), "{ println!(\"{}\", a); }"),
            own(Kind::Node(Node::StmtList), "{ println!(\"{}\", a); }"),
            own(Kind::Token(Token::LCurly), "{"),
            own(Kind::Token(Token::Whitespace), " "),
            own(Kind::Node(Node::ExprStmt), "println!(\"{}\", a);"),
            own(Kind::Node(Node::MacroExpr), "println!(\"{}\", a)"),
            own(Kind::Node(Node::MacroCall), "println!(\"{}\", a)"),
            own(Kind::Node(Node::Path), "println"),
            own(Kind::Node(Node::PathSegment), "println"),
            own(Kind::Node(Node::NameRef), "println"),
            own(Kind::Token(Token::Ident), "println"),
            own(Kind::Token(Token::Bang), "!"),
            own(Kind::Node(Node::TokenTree), "(\"{}\", a)"),
            own(Kind::Token(Token::LParen), "("),
            string("\"{}\"", "{}", Origin::File),
            own(Kind::Token(Token::Comma), ","),
            own(Kind::Token(Token::Whitespace), " "),
            own(Kind::Token(Token::Ident), "a"),
            own(Kind::Token(Token::RParen), ")"),
            own(Kind::Token(Token::Semicolon), ";"),
            own(Kind::Token(Token::Whitespace), " "),
            own(Kind::Token(Token::RCurly), "}"),
            expanded(Kind::Node(Node::Literal), "\"{}\""),
            string(
                "\"{}\"",
                "{}",
                Origin::Expansion(Kind::Node(Node::MacroCall)),
            ),
            expanded(Kind::Node(Node::PathExpr), "a"),
            expanded(Kind::Node(Node::Path), "a"),
            expanded(Kind::Node(Node::PathSegment), "a"),
            expanded(Kind::Node(Node::NameRef), "a"),
            expanded(Kind::Token(Token::Ident), "a"),
        ]),
    },
    Case {
        name: CaseName("a_macro_the_adapter_is_not_told_to_read_again_adds_nothing"),
        word: Word::Succeeds,
        input: || ("println!(\"{}\", a);", Told::Nothing),
        expect: Expect::Value(&[
            own(Kind::Node(Node::SourceFile), "println!(\"{}\", a);"),
            own(Kind::Node(Node::MacroCall), "println!(\"{}\", a);"),
            own(Kind::Node(Node::Path), "println"),
            own(Kind::Node(Node::PathSegment), "println"),
            own(Kind::Node(Node::NameRef), "println"),
            own(Kind::Token(Token::Ident), "println"),
            own(Kind::Token(Token::Bang), "!"),
            own(Kind::Node(Node::TokenTree), "(\"{}\", a)"),
            own(Kind::Token(Token::LParen), "("),
            string("\"{}\"", "{}", Origin::File),
            own(Kind::Token(Token::Comma), ","),
            own(Kind::Token(Token::Whitespace), " "),
            own(Kind::Token(Token::Ident), "a"),
            own(Kind::Token(Token::RParen), ")"),
            own(Kind::Token(Token::Semicolon), ";"),
        ]),
    },
];

/// The contract's rows for string literals: the source, and the value the adapter decoded for each string token of it, in order; no
/// value where the library rejects an escape.
pub(crate) const LITERALS: &[Case<&'static str, &'static [Decoding], Failure>] = &[
    Case {
        name: CaseName("a_plain_string_is_its_text"),
        word: Word::Succeeds,
        input: || "const C: &str = \"x/y.rs\";",
        expect: Expect::Value(&[Decoding::Decoded("x/y.rs")]),
    },
    Case {
        name: CaseName("simple_escapes_are_resolved"),
        word: Word::Succeeds,
        input: || "const C: &str = \"a\\n\\t\\\"b\\\\\";",
        expect: Expect::Value(&[Decoding::Decoded("a\n\t\"b\\")]),
    },
    Case {
        name: CaseName("a_unicode_escape_is_resolved"),
        word: Word::Succeeds,
        input: || "const C: &str = \"\\u{41}\";",
        expect: Expect::Value(&[Decoding::Decoded("A")]),
    },
    Case {
        name: CaseName("a_raw_string_is_its_text_as_written"),
        word: Word::Succeeds,
        input: || "const C: &str = r#\"a\\n\"b\"#;",
        expect: Expect::Value(&[Decoding::Decoded("a\\n\"b")]),
    },
    Case {
        name: CaseName("an_escape_that_is_not_one_has_no_value"),
        word: Word::Succeeds,
        input: || "const C: &str = \"\\q\";",
        expect: Expect::Value(&[Decoding::Plain]),
    },
    Case {
        name: CaseName("every_string_of_a_file_has_its_own_value"),
        word: Word::Succeeds,
        input: || "const A: &str = \"a\"; const B: &str = \"b\\n\";",
        expect: Expect::Value(&[Decoding::Decoded("a"), Decoding::Decoded("b\n")]),
    },
];

/// The macros an adapter built as `told` reads again.
#[must_use]
pub(crate) const fn expanding(told: Told) -> Expanding<'static> {
    match told {
        Told::Standard => Expanding(&EXPANDED_MACROS),
        Told::Nothing => Expanding(&[]),
    }
}

/// Every element of the table `adapter` reads `source` into, in order: where it comes from, its kind and the text it spans.
#[must_use]
pub(crate) fn made_by(adapter: &impl SyntaxPort, source: &str) -> Vec<Made> {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    adapter.read(
        SourceText(source.as_bytes()),
        &mut table_builder(&mut elements, &mut symbols, ByteSlice(source.as_bytes())),
    );
    positions_of(&elements, source)
}

/// Every element of `elements`, read from `source`, in order.
fn positions_of(elements: &Elements, source: &str) -> Vec<Made> {
    (0..end_of(elements).0)
        .map(Position)
        .map(|position| {
            let (start, end) = (
                usize::try_from(start_at(elements, position).0).unwrap_or(0),
                usize::try_from(end_at(elements, position).0).unwrap_or(0),
            );
            Made {
                origin: match origin_at(elements, position) {
                    ElementOrigin::File => Origin::File,
                    ElementOrigin::Implied => Origin::Implied,
                    ElementOrigin::Expansion { call_site, .. } => {
                        Origin::Expansion(kind_of(kind_at(elements, call_site)))
                    }
                },
                element: Want {
                    kind: kind_of(kind_at(elements, position)),
                    text: Box::leak(source.get(start..end).unwrap_or_default().into()),
                },
                value: decoded_at(elements, position).map_or(Decoding::Plain, |text| {
                    Decoding::Decoded(Box::leak(text.0.to_owned().into_boxed_str()))
                }),
            }
        })
        .collect()
}

/// Runs the contract's rows against one implementation, built by `adapter` for the macros it is told to read again: an adapter that
/// reads source text.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub(crate) fn check<P>(adapter: impl Fn(Expanding<'static>) -> P)
where
    P: SyntaxPort,
{
    run(CASES, Word::Succeeds, |(source, told)| {
        let made = made_by(&adapter(expanding(told)), source);
        let leaked: &'static [Made] = Box::leak(made.into_boxed_slice());
        Ok(leaked)
    });
    run(LITERALS, Word::Succeeds, |source| {
        let values: Vec<Decoding> = made_by(&adapter(expanding(Told::Nothing)), source)
            .into_iter()
            .filter(|made| made.element.kind == Kind::Token(Token::String))
            .map(|made| made.value)
            .collect();
        let leaked: &'static [Decoding] = Box::leak(values.into_boxed_slice());
        Ok(leaked)
    });
}

#[cfg(test)]
#[path = "contract_test.rs"]
mod tests;
