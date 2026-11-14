//! The `shapes` service's cases: which written differences leave two nodes one class, and which make two.

use super::service::prepare;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::model::types::ReadIndexes;
use crate::domain::syntax::{service::is_token, types::Leaf};
use crate::outbound::syntax::{
    factory,
    interface::SyntaxPort as _,
    types::{Expanding, SourceText},
};
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
        input: || ("fn f() { g(x); g(x); }", "g(x)", "g(x)"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("parentheses_leave_the_class"),
        word: Word::Succeeds,
        input: || ("fn f() { g((a)); g(a); }", "g((a))", "g(a)"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_number_is_its_value_not_its_spelling"),
        word: Word::Succeeds,
        input: || ("fn f() { g(16); g(0x1_0); }", "16", "0x1_0"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_suffix_makes_the_class"),
        word: Word::Succeeds,
        input: || ("fn f() { g(1u8); g(1u16); }", "1u8", "1u16"),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("an_integer_is_not_the_float_it_writes"),
        word: Word::Succeeds,
        input: || ("fn f() { g(1); g(1.0); }", "1", "1.0"),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("a_string_is_its_value_not_its_quotes"),
        word: Word::Succeeds,
        input: || ("fn f() { g(\"a\"); g(r\"a\"); }", "\"a\"", "r\"a\""),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("fields_of_different_names_differ"),
        word: Word::Succeeds,
        input: || ("fn f() { g(a.b); g(a.c); }", "a.b", "a.c"),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("a_comment_inside_leaves_the_class"),
        word: Word::Succeeds,
        input: || ("fn f() { g(/* c */ x); g(x); }", "g(/* c */ x)", "g(x)"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("operators_make_the_class"),
        word: Word::Succeeds,
        input: || ("fn f() { g(a < b); g(a > b); }", "a < b", "a > b"),
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
    factory::ra_ap_syntax(Expanding(&[])).read(
        SourceText(source.as_bytes()),
        &mut languages::table_builder(&mut elements, &mut symbols, bytes),
    );
    let tree = languages::Tree {
        elements: &elements,
        source: bytes,
        file: languages::FileId(0),
    };
    let spelled = |wanted: &str| {
        languages::positions(tree.elements)
            .filter(|at| {
                let from = usize::try_from(languages::start_at(tree.elements, *at).0)
                    .expect("an offset is an index");
                let to = usize::try_from(languages::end_at(tree.elements, *at).0)
                    .expect("an offset is an index");
                is_token(&tree, *at) == Leaf::Node && source.get(from..to) == Some(wanted)
            })
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
        languages::key_class_at(&index, first) == languages::key_class_at(&index, last)
            && languages::key_class_at(&index, first).is_some(),
    )
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
