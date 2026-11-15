//! The `string_literals` service's cases: the token a string literal of a parsed file is written with, and where a string token is
//! not a literal.

use super::service::string_token;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::syntax::service::{end, start};
use crate::outbound::syntax::{
    factory,
    interface::SyntaxPort as _,
    types::{Expanding, SourceText},
};
use ryo::languages;

/// The source a row parses.
type Input = &'static str;
/// The written text of each string token the service answers, in the order the file writes them, joined by a space.
type Value = &'static str;
/// Reading a tree is never refused.
type Failure = ();

/// The rows of this concept's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_literal_expression_is_written_with_its_string"),
        word: Word::Succeeds,
        input: || "const A: &str = \"a\";",
        expect: Expect::Value("\"a\""),
    },
    Case {
        name: CaseName("a_string_in_an_attribute_s_arguments_is_a_literal"),
        word: Word::Succeeds,
        input: || "#[rustfmt::skip(v = r\"b\")]\nfn f() {}",
        expect: Expect::Value("r\"b\""),
    },
    Case {
        name: CaseName("a_string_in_a_cfg_predicate_or_an_abi_is_a_literal"),
        word: Word::Succeeds,
        input: || "#[cfg(feature = \"c\")]\nextern \"C\" fn f() {}",
        expect: Expect::Value("\"c\" \"C\""),
    },
    Case {
        name: CaseName("a_string_in_another_macro_s_input_is_not_a_literal"),
        word: Word::Succeeds,
        input: || "fn f() { other!(\"d\"); }",
        expect: Expect::Value(""),
    },
];

/// Parses the row's source and writes out the string tokens the service answers for its elements.
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
    let mut written: Vec<&str> = languages::positions(tree.elements)
        .filter_map(|at| string_token(&tree, at))
        .map(|token| &source[start(&tree, token).0 as usize..end(&tree, token).0 as usize])
        .collect();
    written.dedup();
    Ok(Box::leak(written.join(" ").into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
