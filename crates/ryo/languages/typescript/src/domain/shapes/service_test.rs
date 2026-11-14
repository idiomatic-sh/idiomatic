//! The `shapes` service's cases: which written differences leave two nodes one class, and which make two.

use super::service::prepare;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::model::types::ReadIndexes;
use crate::domain::names::service::text;
use crate::domain::syntax::{
    service::is_token,
    types::{Dialect, Leaf},
};
use crate::outbound::syntax::{factory, interface::SyntaxPort as _, types::SourceText};
use ryo::languages;

/// The source a row parses, and the spellings of the two nodes it compares: the first node spelled as the one, the last spelled
/// as the other.
type Input = (&'static str, &'static str, &'static str);
/// Whether the two nodes share a class.
type Value = bool;
/// Reading a tree is never refused.
type Failure = ();

/// The rows of this concept's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("equal_calls_share_a_class"),
        word: Word::Succeeds,
        input: || ("f(x);\nf(x);\n", "f(x)", "f(x)"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("parentheses_leave_the_class"),
        word: Word::Succeeds,
        input: || ("(a);\na;\n", "(a);", "a;"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_number_is_its_value_not_its_spelling"),
        word: Word::Succeeds,
        input: || ("1;\n1.0;\n", "1", "1.0"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_hexadecimal_number_is_its_value"),
        word: Word::Succeeds,
        input: || ("0x1;\n1;\n", "0x1", "1"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_bigint_is_not_the_number_it_writes"),
        word: Word::Succeeds,
        input: || ("1n;\n1;\n", "1n", "1"),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("a_string_is_its_value_not_its_quotes"),
        word: Word::Succeeds,
        input: || ("'a';\n\"a\";\n", "'a'", "\"a\""),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("members_of_different_names_differ"),
        word: Word::Succeeds,
        input: || ("a.b;\na.c;\n", "a.b", "a.c"),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("a_comment_inside_leaves_the_class"),
        word: Word::Succeeds,
        input: || ("f(/* c */ x);\nf(x);\n", "f(/* c */ x)", "f(x)"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("operators_make_the_class"),
        word: Word::Succeeds,
        input: || ("a < b;\na > b;\n", "a < b", "a > b"),
        expect: Expect::Value(false),
    },
];

/// Parses the row's source and answers whether its two named nodes share a class.
fn subject((source, left, right): Input) -> Result<Value, Failure> {
    let (mut elements, mut symbols) = (
        languages::Elements::default(),
        languages::Symbols::default(),
    );
    let bytes = languages::ByteSlice(source.as_bytes());
    factory::oxc_parser().read(
        SourceText(source.as_bytes()),
        Dialect::TypeScript,
        &mut languages::table_builder(&mut elements, &mut symbols, bytes),
    );
    let tree = languages::Tree {
        elements: &elements,
        source: bytes,
        file: languages::FileId(0),
    };
    let spelled = |wanted: &str| {
        languages::positions(tree.elements)
            .filter(|at| is_token(&tree, *at) == Leaf::Node && text(&tree, *at).0 == wanted)
            .collect::<Vec<_>>()
    };
    let first = *spelled(left)
        .first()
        .expect("the row's source writes the left node");
    let last = *spelled(right)
        .last()
        .expect("the row's source writes the right node");
    let indexes = ReadIndexes::default();
    let index = prepare(&tree, &indexes, &mut symbols).expect("a complete tree has classes");
    Ok(
        languages::shape_class_at(&index, first) == languages::shape_class_at(&index, last)
            && languages::shape_class_at(&index, first).is_some(),
    )
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
