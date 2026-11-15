//! The `attribute_paths` service's cases: the paths a parsed file's attributes write, each with the name it is written as.

use super::service::{path_name, written_paths};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::syntax::{
    factory,
    interface::SyntaxPort as _,
    types::{Expanding, SourceText},
};
use ryo::languages;

/// The source a row parses.
type Input = &'static str;
/// The name of each path the service answers, in the order the file writes them, joined by a space.
type Value = &'static str;
/// Reading a tree is never refused.
type Failure = ();

/// The rows of this concept's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_meta_path_is_its_segments_without_a_comment"),
        word: Word::Succeeds,
        input: || "#[rustfmt /* c */ ::skip]\nfn f() {}",
        expect: Expect::Value("rustfmt::skip"),
    },
    Case {
        name: CaseName("a_token_tree_writes_each_path_without_a_comment"),
        word: Word::Succeeds,
        input: || "#[allow(clippy:: /* c */ len_zero, dead_code)]\nfn f() {}",
        expect: Expect::Value("allow clippy::len_zero dead_code"),
    },
    Case {
        name: CaseName("a_raw_segment_and_a_leading_separator_are_kept"),
        word: Word::Succeeds,
        input: || "#[rustfmt::skip(r#a, ::b::r#c)]\nfn f() {}",
        expect: Expect::Value("rustfmt::skip r#a ::b::r#c"),
    },
    Case {
        name: CaseName("a_path_outside_an_attribute_is_not_an_attribute_path"),
        word: Word::Succeeds,
        input: || "fn f() { a::b(); other!(c::d); }",
        expect: Expect::Value(""),
    },
];

/// Parses the row's source and writes out the name of each path the service answers.
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
    let paths: Vec<_> = languages::positions(tree.elements)
        .flat_map(|at| written_paths(&tree, at))
        .collect();
    let names: Vec<String> = paths
        .into_iter()
        .map(|path| {
            let name = path_name(&tree, &mut symbols, path);
            languages::text_of(&symbols, tree.source, name).map_or_else(String::new, |text| text.0.to_owned())
        })
        .collect();
    Ok(Box::leak(names.join(" ").into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
