//! The `numeric_literals` service's cases: what a literal node of a parsed file holds of a number.

use super::service::stored;
use super::types::LiteralNumber;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::{service::kind, types::Kind};
use crate::outbound::syntax::{
    factory,
    interface::SyntaxPort as _,
    types::{Expanding, SourceText},
};
use ryo::languages;

/// The source a row parses, with one literal in it.
type Input = &'static str;
/// What the literal node holds of a number, written out.
type Value = &'static str;
/// Reading a tree is never refused.
type Failure = ();

/// The rows of this concept's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_number_holds_its_kind_and_value"),
        word: Word::Succeeds,
        input: || "const A: u8 = 0x10;",
        expect: Expect::Value("integer 16"),
    },
    Case {
        name: CaseName("a_string_is_no_number"),
        word: Word::Succeeds,
        input: || "const A: &str = \"16\";",
        expect: Expect::Value("no number"),
    },
    Case {
        name: CaseName("a_number_the_compiler_refuses_holds_no_value"),
        word: Word::Succeeds,
        input: || "const A: u8 = 0b102;",
        expect: Expect::Value("unread"),
    },
];

/// Parses the row's source and writes out what its literal node holds.
fn subject(source: Input) -> Result<Value, Failure> {
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
    let literal = languages::positions(tree.elements)
        .find(|at| kind(&tree, *at) == Kind::Node(Node::Literal))
        .expect("the row's source writes a literal");
    let written = match stored(&tree, literal) {
        LiteralNumber::NotNumeric => "no number".to_owned(),
        LiteralNumber::Unread => "unread".to_owned(),
        LiteralNumber::Known { kind, value } => format!("{} {}", kind.0, value.0),
    };
    Ok(Box::leak(written.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
