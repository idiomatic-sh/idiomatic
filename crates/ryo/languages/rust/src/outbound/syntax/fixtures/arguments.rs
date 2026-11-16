//! What every `syntax` adapter must pass for the arguments of macro calls: the elements it adds for them, each with the call it is
//! under, compared in full with elements written by hand. The file's own elements are the rows of `contract.rs`.

use crate::cases::{Case, CaseName, Expect, Word};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::kind_of;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::SourceText;
use ryo::languages::{
    ByteSlice, ElementOrigin, Elements, Position, Symbols, end_at, end_of, kind_at, origin_at,
    start_at, table_builder,
};

/// An element the adapter added for the arguments of a macro call: the text of the call it is under, and its kind and text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Added {
    /// The text of the call whose arguments it was read from.
    pub call: &'static str,
    /// Its kind.
    pub kind: Kind,
    /// The text it spans.
    pub text: &'static str,
}

/// The call of the `matches!` row.
const SCRUTINEE_PATTERN_AND_GUARD: &str = "matches!(f(a, b), Some(x) if x > 0)";

/// An element added under the call written `call`.
const fn under(call: &'static str, kind: Kind, text: &'static str) -> Added {
    Added { call, kind, text }
}

/// The rows: a name, one case word, Rust source text read with the language's opaque macros, and the elements added for it, in order.
pub(crate) const ARGUMENTS: &[Case<&'static str, &'static [Added], ()>] = &[
    Case {
        name: CaseName("a_call_of_a_user_macro_has_its_arguments_under_it"),
        word: Word::Succeeds,
        input: || "m!(a, b);",
        expect: Expect::Value(&[
            under("m!(a, b);", Kind::Node(Node::PathExpr), "a"),
            under("m!(a, b);", Kind::Node(Node::Path), "a"),
            under("m!(a, b);", Kind::Node(Node::PathSegment), "a"),
            under("m!(a, b);", Kind::Node(Node::NameRef), "a"),
            under("m!(a, b);", Kind::Token(Token::Ident), "a"),
            under("m!(a, b);", Kind::Node(Node::PathExpr), "b"),
            under("m!(a, b);", Kind::Node(Node::Path), "b"),
            under("m!(a, b);", Kind::Node(Node::PathSegment), "b"),
            under("m!(a, b);", Kind::Node(Node::NameRef), "b"),
            under("m!(a, b);", Kind::Token(Token::Ident), "b"),
        ]),
    },
    Case {
        name: CaseName("a_closure_with_two_parameters_is_one_argument"),
        word: Word::Succeeds,
        input: || "m!(n, |x, y| x);",
        expect: Expect::Value(&[
            under("m!(n, |x, y| x);", Kind::Node(Node::PathExpr), "n"),
            under("m!(n, |x, y| x);", Kind::Node(Node::Path), "n"),
            under("m!(n, |x, y| x);", Kind::Node(Node::PathSegment), "n"),
            under("m!(n, |x, y| x);", Kind::Node(Node::NameRef), "n"),
            under("m!(n, |x, y| x);", Kind::Token(Token::Ident), "n"),
            under(
                "m!(n, |x, y| x);",
                Kind::Node(Node::ClosureExpr),
                "|x, y| x",
            ),
            under("m!(n, |x, y| x);", Kind::Node(Node::ParamList), "|x, y|"),
            under("m!(n, |x, y| x);", Kind::Token(Token::Pipe), "|"),
            under("m!(n, |x, y| x);", Kind::Node(Node::Param), "x"),
            under("m!(n, |x, y| x);", Kind::Node(Node::IdentPat), "x"),
            under("m!(n, |x, y| x);", Kind::Node(Node::Name), "x"),
            under("m!(n, |x, y| x);", Kind::Token(Token::Ident), "x"),
            under("m!(n, |x, y| x);", Kind::Token(Token::Comma), ","),
            under("m!(n, |x, y| x);", Kind::Token(Token::Whitespace), " "),
            under("m!(n, |x, y| x);", Kind::Node(Node::Param), "y"),
            under("m!(n, |x, y| x);", Kind::Node(Node::IdentPat), "y"),
            under("m!(n, |x, y| x);", Kind::Node(Node::Name), "y"),
            under("m!(n, |x, y| x);", Kind::Token(Token::Ident), "y"),
            under("m!(n, |x, y| x);", Kind::Token(Token::Pipe), "|"),
            under("m!(n, |x, y| x);", Kind::Token(Token::Whitespace), " "),
            under("m!(n, |x, y| x);", Kind::Node(Node::PathExpr), "x"),
            under("m!(n, |x, y| x);", Kind::Node(Node::Path), "x"),
            under("m!(n, |x, y| x);", Kind::Node(Node::PathSegment), "x"),
            under("m!(n, |x, y| x);", Kind::Node(Node::NameRef), "x"),
            under("m!(n, |x, y| x);", Kind::Token(Token::Ident), "x"),
        ]),
    },
    Case {
        name: CaseName("a_named_argument_is_read_as_its_value"),
        word: Word::Succeeds,
        input: || "m!(a, width = 3);",
        expect: Expect::Value(&[
            under("m!(a, width = 3);", Kind::Node(Node::PathExpr), "a"),
            under("m!(a, width = 3);", Kind::Node(Node::Path), "a"),
            under("m!(a, width = 3);", Kind::Node(Node::PathSegment), "a"),
            under("m!(a, width = 3);", Kind::Node(Node::NameRef), "a"),
            under("m!(a, width = 3);", Kind::Token(Token::Ident), "a"),
            under("m!(a, width = 3);", Kind::Node(Node::Literal), "3"),
            under("m!(a, width = 3);", Kind::Token(Token::IntNumber), "3"),
        ]),
    },
    Case {
        name: CaseName("a_braced_call_has_its_arguments_under_it"),
        word: Word::Succeeds,
        input: || "m! { a }",
        expect: Expect::Value(&[
            under("m! { a }", Kind::Node(Node::PathExpr), "a"),
            under("m! { a }", Kind::Node(Node::Path), "a"),
            under("m! { a }", Kind::Node(Node::PathSegment), "a"),
            under("m! { a }", Kind::Node(Node::NameRef), "a"),
            under("m! { a }", Kind::Token(Token::Ident), "a"),
        ]),
    },
    Case {
        name: CaseName("a_parenthesized_argument_keeps_its_parentheses"),
        word: Word::Succeeds,
        input: || "m!((a));",
        expect: Expect::Value(&[
            under("m!((a));", Kind::Node(Node::ParenExpr), "(a)"),
            under("m!((a));", Kind::Token(Token::LParen), "("),
            under("m!((a));", Kind::Node(Node::PathExpr), "a"),
            under("m!((a));", Kind::Node(Node::Path), "a"),
            under("m!((a));", Kind::Node(Node::PathSegment), "a"),
            under("m!((a));", Kind::Node(Node::NameRef), "a"),
            under("m!((a));", Kind::Token(Token::Ident), "a"),
            under("m!((a));", Kind::Token(Token::RParen), ")"),
        ]),
    },
    Case {
        name: CaseName("vec_reads_its_element_and_its_count"),
        word: Word::Succeeds,
        input: || "vec![a; 2];",
        expect: Expect::Value(&[
            under("vec![a; 2];", Kind::Node(Node::PathExpr), "a"),
            under("vec![a; 2];", Kind::Node(Node::Path), "a"),
            under("vec![a; 2];", Kind::Node(Node::PathSegment), "a"),
            under("vec![a; 2];", Kind::Node(Node::NameRef), "a"),
            under("vec![a; 2];", Kind::Token(Token::Ident), "a"),
            under("vec![a; 2];", Kind::Node(Node::Literal), "2"),
            under("vec![a; 2];", Kind::Token(Token::IntNumber), "2"),
        ]),
    },
    Case {
        name: CaseName("matches_reads_its_scrutinee_pattern_and_guard"),
        word: Word::Succeeds,
        input: || "matches!(f(a, b), Some(x) if x > 0)",
        expect: Expect::Value(&[
            under(
                SCRUTINEE_PATTERN_AND_GUARD,
                Kind::Node(Node::CallExpr),
                "f(a, b)",
            ),
            under(SCRUTINEE_PATTERN_AND_GUARD, Kind::Node(Node::PathExpr), "f"),
            under(SCRUTINEE_PATTERN_AND_GUARD, Kind::Node(Node::Path), "f"),
            under(
                SCRUTINEE_PATTERN_AND_GUARD,
                Kind::Node(Node::PathSegment),
                "f",
            ),
            under(SCRUTINEE_PATTERN_AND_GUARD, Kind::Node(Node::NameRef), "f"),
            under(SCRUTINEE_PATTERN_AND_GUARD, Kind::Token(Token::Ident), "f"),
            under(
                SCRUTINEE_PATTERN_AND_GUARD,
                Kind::Node(Node::ArgList),
                "(a, b)",
            ),
            under(SCRUTINEE_PATTERN_AND_GUARD, Kind::Token(Token::LParen), "("),
            under(SCRUTINEE_PATTERN_AND_GUARD, Kind::Node(Node::PathExpr), "a"),
            under(SCRUTINEE_PATTERN_AND_GUARD, Kind::Node(Node::Path), "a"),
            under(
                SCRUTINEE_PATTERN_AND_GUARD,
                Kind::Node(Node::PathSegment),
                "a",
            ),
            under(SCRUTINEE_PATTERN_AND_GUARD, Kind::Node(Node::NameRef), "a"),
            under(SCRUTINEE_PATTERN_AND_GUARD, Kind::Token(Token::Ident), "a"),
            under(SCRUTINEE_PATTERN_AND_GUARD, Kind::Token(Token::Comma), ","),
            under(
                SCRUTINEE_PATTERN_AND_GUARD,
                Kind::Token(Token::Whitespace),
                " ",
            ),
            under(SCRUTINEE_PATTERN_AND_GUARD, Kind::Node(Node::PathExpr), "b"),
            under(SCRUTINEE_PATTERN_AND_GUARD, Kind::Node(Node::Path), "b"),
            under(
                SCRUTINEE_PATTERN_AND_GUARD,
                Kind::Node(Node::PathSegment),
                "b",
            ),
            under(SCRUTINEE_PATTERN_AND_GUARD, Kind::Node(Node::NameRef), "b"),
            under(SCRUTINEE_PATTERN_AND_GUARD, Kind::Token(Token::Ident), "b"),
            under(SCRUTINEE_PATTERN_AND_GUARD, Kind::Token(Token::RParen), ")"),
            under(
                SCRUTINEE_PATTERN_AND_GUARD,
                Kind::Node(Node::TupleStructPat),
                "Some(x)",
            ),
            under(SCRUTINEE_PATTERN_AND_GUARD, Kind::Node(Node::Path), "Some"),
            under(
                SCRUTINEE_PATTERN_AND_GUARD,
                Kind::Node(Node::PathSegment),
                "Some",
            ),
            under(
                SCRUTINEE_PATTERN_AND_GUARD,
                Kind::Node(Node::NameRef),
                "Some",
            ),
            under(
                SCRUTINEE_PATTERN_AND_GUARD,
                Kind::Token(Token::Ident),
                "Some",
            ),
            under(SCRUTINEE_PATTERN_AND_GUARD, Kind::Token(Token::LParen), "("),
            under(SCRUTINEE_PATTERN_AND_GUARD, Kind::Node(Node::IdentPat), "x"),
            under(SCRUTINEE_PATTERN_AND_GUARD, Kind::Node(Node::Name), "x"),
            under(SCRUTINEE_PATTERN_AND_GUARD, Kind::Token(Token::Ident), "x"),
            under(SCRUTINEE_PATTERN_AND_GUARD, Kind::Token(Token::RParen), ")"),
            under(
                SCRUTINEE_PATTERN_AND_GUARD,
                Kind::Node(Node::BinExpr),
                "x > 0",
            ),
            under(SCRUTINEE_PATTERN_AND_GUARD, Kind::Node(Node::PathExpr), "x"),
            under(SCRUTINEE_PATTERN_AND_GUARD, Kind::Node(Node::Path), "x"),
            under(
                SCRUTINEE_PATTERN_AND_GUARD,
                Kind::Node(Node::PathSegment),
                "x",
            ),
            under(SCRUTINEE_PATTERN_AND_GUARD, Kind::Node(Node::NameRef), "x"),
            under(SCRUTINEE_PATTERN_AND_GUARD, Kind::Token(Token::Ident), "x"),
            under(
                SCRUTINEE_PATTERN_AND_GUARD,
                Kind::Token(Token::Whitespace),
                " ",
            ),
            under(SCRUTINEE_PATTERN_AND_GUARD, Kind::Token(Token::RAngle), ">"),
            under(
                SCRUTINEE_PATTERN_AND_GUARD,
                Kind::Token(Token::Whitespace),
                " ",
            ),
            under(SCRUTINEE_PATTERN_AND_GUARD, Kind::Node(Node::Literal), "0"),
            under(
                SCRUTINEE_PATTERN_AND_GUARD,
                Kind::Token(Token::IntNumber),
                "0",
            ),
        ]),
    },
    Case {
        name: CaseName("a_call_inside_an_argument_has_its_arguments_under_it"),
        word: Word::Succeeds,
        input: || "m!(|| n!(a));",
        expect: Expect::Value(&[
            under("m!(|| n!(a));", Kind::Node(Node::ClosureExpr), "|| n!(a)"),
            under("m!(|| n!(a));", Kind::Node(Node::ParamList), "||"),
            under("m!(|| n!(a));", Kind::Token(Token::Pipe), "|"),
            under("m!(|| n!(a));", Kind::Token(Token::Pipe), "|"),
            under("m!(|| n!(a));", Kind::Token(Token::Whitespace), " "),
            under("m!(|| n!(a));", Kind::Node(Node::MacroExpr), "n!(a)"),
            under("m!(|| n!(a));", Kind::Node(Node::MacroCall), "n!(a)"),
            under("m!(|| n!(a));", Kind::Node(Node::Path), "n"),
            under("m!(|| n!(a));", Kind::Node(Node::PathSegment), "n"),
            under("m!(|| n!(a));", Kind::Node(Node::NameRef), "n"),
            under("m!(|| n!(a));", Kind::Token(Token::Ident), "n"),
            under("m!(|| n!(a));", Kind::Token(Token::Bang), "!"),
            under("m!(|| n!(a));", Kind::Node(Node::TokenTree), "(a)"),
            under("m!(|| n!(a));", Kind::Token(Token::LParen), "("),
            under("m!(|| n!(a));", Kind::Token(Token::Ident), "a"),
            under("m!(|| n!(a));", Kind::Token(Token::RParen), ")"),
            under("n!(a)", Kind::Node(Node::PathExpr), "a"),
            under("n!(a)", Kind::Node(Node::Path), "a"),
            under("n!(a)", Kind::Node(Node::PathSegment), "a"),
            under("n!(a)", Kind::Node(Node::NameRef), "a"),
            under("n!(a)", Kind::Token(Token::Ident), "a"),
        ]),
    },
    Case {
        name: CaseName("a_built_in_macro_that_takes_no_code_adds_nothing"),
        word: Word::Succeeds,
        input: || "stringify!(a + b); cfg!(feature = \"x\");",
        expect: Expect::Value(&[]),
    },
    Case {
        name: CaseName("a_token_tree_that_is_not_a_list_of_expressions_adds_nothing"),
        word: Word::Succeeds,
        input: || "m!(static X: u8 = 1;); m!(a, static);",
        expect: Expect::Value(&[]),
    },
    Case {
        name: CaseName("a_call_written_inside_a_token_tree_is_not_read"),
        word: Word::Succeeds,
        input: || "macro_rules! r { () => { println!(\"{}\", 1) } }",
        expect: Expect::Value(&[]),
    },
    Case {
        name: CaseName("an_unclosed_call_adds_nothing"),
        word: Word::Succeeds,
        input: || "fn f() { m!(é",
        expect: Expect::Value(&[]),
    },
];

/// Every element `adapter` added for the macro calls of `source`, in order, with the text of the call it is under.
#[must_use]
pub(crate) fn added_by(adapter: &impl SyntaxPort, source: &'static str) -> Vec<Added> {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    adapter.read(
        SourceText(source.as_bytes()),
        &mut table_builder(&mut elements, &mut symbols, ByteSlice(source.as_bytes())),
    );
    added(&elements, source)
}

/// The elements of `elements` that a macro call's arguments were read into, read from `source`.
fn added(elements: &Elements, source: &'static str) -> Vec<Added> {
    let text = |position: Position| {
        let (start, end) = (
            usize::try_from(start_at(elements, position).0).unwrap_or(0),
            usize::try_from(end_at(elements, position).0).unwrap_or(0),
        );
        source.get(start..end).unwrap_or_default()
    };
    (0..end_of(elements).0)
        .map(Position)
        .filter_map(|position| match origin_at(elements, position) {
            ElementOrigin::Expansion { call_site, .. } => Some(Added {
                call: text(call_site),
                kind: kind_of(kind_at(elements, position)),
                text: text(position),
            }),
            ElementOrigin::File | ElementOrigin::Implied | ElementOrigin::Embedded { .. } => None,
        })
        .collect()
}
