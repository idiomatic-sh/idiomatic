//! The `cssparser` adapter of the syntax port: the port's contract rows run through it, then the cases specific to it.

use super::adapter::SyntaxCssparserAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::syntax::service::kind_name;
use crate::outbound::syntax::fixtures::contract::{check, elements_of};
use crate::outbound::syntax::interface::SyntaxPort as _;
use ryo::languages::Elements;
use super::fixtures::projection::{differences, dumped, projected};







/// What a row reads: CSS text whose elements are listed, a file and the tree the standard's tool printed for it, or every file
/// whose tree is pinned.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The elements of the text.
    Tree(&'static str),
    /// A file and what tinycss2 printed for it: our tree has the standard's nodes and tokens.
    Standard(&'static str, &'static str),
    /// Every relation's input, and every file of the corpus and of the suite, whose tree is pinned in `standard/`.
    Pinned,
}

type Input = Given;
/// The kind of each element the adapter read the text into, in order; what differs from the standard's tree; or the files whose tree
/// is not the standard's, by path below `tests/fixtures`.
type Value = Seen;
type Failure = ();

/// What a case looks at.
#[derive(Debug, PartialEq, Eq)]
enum Seen {
    /// The kinds of the elements, in order.
    Elements(&'static str),
    /// The first place our tree differs from the standard's; nothing when it does not.
    Standard(Option<&'static str>),
    /// The pinned files whose tree differs from the standard's.
    Pinned(&'static [&'static str]),
}












/// The rows of what is specific to this adapter.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_rule_holds_its_prelude_and_its_declarations"),
        word: Word::Succeeds,
        input: || Given::Tree("a { color: red; }"),
        expect: Expect::Value(Seen::Elements(
            "stylesheet qualified-rule ident-token whitespace-token {-token whitespace-token declaration ident-token colon-token whitespace-token ident-token semicolon-token whitespace-token }-token",
        )),
    },
    Case {
        name: CaseName("an_import_is_an_at_rule_whose_prelude_is_its_url"),
        word: Word::Succeeds,
        input: || Given::Tree("@import url(\"a.css\") screen;"),
        expect: Expect::Value(Seen::Elements(
            "stylesheet at-rule at-keyword-token whitespace-token function function-token string-token )-token whitespace-token ident-token semicolon-token",
        )),
    },
    Case {
        name: CaseName("important_is_two_tokens_of_the_declaration"),
        word: Word::Succeeds,
        input: || Given::Tree(".a { color: red !important }"),
        expect: Expect::Value(Seen::Elements(
            "stylesheet qualified-rule delim-token ident-token whitespace-token {-token whitespace-token declaration ident-token colon-token whitespace-token ident-token whitespace-token delim-token ident-token whitespace-token }-token",
        )),
    },
    Case {
        name: CaseName("a_conditional_rule_holds_rules"),
        word: Word::Succeeds,
        input: || Given::Tree("@media screen { a {} }"),
        expect: Expect::Value(Seen::Elements(
            "stylesheet at-rule at-keyword-token whitespace-token ident-token whitespace-token {-token whitespace-token qualified-rule ident-token whitespace-token {-token }-token whitespace-token }-token",
        )),
    },
    Case {
        name: CaseName("a_declaration_holding_a_block_is_a_nested_rule"),
        word: Word::Succeeds,
        input: || Given::Tree(".a { b:hover {} }"),
        expect: Expect::Value(Seen::Elements(
            "stylesheet qualified-rule delim-token ident-token whitespace-token {-token whitespace-token qualified-rule ident-token colon-token ident-token whitespace-token {-token }-token whitespace-token }-token",
        )),
    },
    Case {
        name: CaseName("an_unknown_at_rule_holds_component_values"),
        word: Word::Succeeds,
        input: || Given::Tree("@foo { bar }"),
        expect: Expect::Value(Seen::Elements(
            "stylesheet at-rule at-keyword-token whitespace-token {-token whitespace-token ident-token whitespace-token }-token",
        )),
    },
    Case {
        name: CaseName("a_rule_without_a_block_is_still_in_the_tree"),
        word: Word::Succeeds,
        input: || Given::Tree("foo 4"),
        expect: Expect::Value(Seen::Elements(
            "stylesheet qualified-rule ident-token whitespace-token number-token",
        )),
    },
    Case {
        name: CaseName("a_byte_order_mark_is_a_token_of_its_own"),
        word: Word::Succeeds,
        input: || Given::Tree("\u{feff}a{}"),
        expect: Expect::Value(Seen::Elements(
            "stylesheet byte-order-mark qualified-rule ident-token {-token }-token",
        )),
    },
    Case {
        name: CaseName("an_attribute_matcher_is_two_delim_tokens_in_a_simple_block"),
        word: Word::Succeeds,
        input: || Given::Tree("[a~=b]{}"),
        expect: Expect::Value(Seen::Elements(
            "stylesheet qualified-rule simple-block [-token ident-token delim-token delim-token ident-token ]-token {-token }-token",
        )),
    },
    Case {
        name: CaseName("a_tree_that_is_not_the_standard_s_is_a_difference"),
        word: Word::Succeeds,
        input: || {
            Given::Standard(
                "a { color: red }",
                "stylesheet\n  at-rule media\n    block\n",
            )
        },
        expect: Expect::Value(Seen::Standard(Some(
            "/stylesheet[0]: we have `qualified-rule`, the standard has `at-rule`",
        ))),
    },
    Case {
        name: CaseName("every_pinned_relation_corpus_and_suite_file_has_the_standard_s_tree"),
        word: Word::Succeeds,
        input: || Given::Pinned,
        // The one exception: `@foo; @charset 4 {}`. An `@charset` that is not the first rule is refused by the library, which recovers
        // from it and keeps its tokens without a node, where tinycss2 keeps the at-rule.
        expect: Expect::Value(Seen::Pinned(&["suite/invalid/stylesheet/05.css"])),
    },
];

/// The trees pinned below `standard`, each with the file it was printed for, both by path below `fixtures`, sorted: for the corpus
/// and the suite at the same path, and for each relation's input beside the page `standard/<relation>.tree`.
fn pinned(
    standard: &std::path::Path,
    fixtures: &std::path::Path,
) -> Vec<(std::path::PathBuf, std::path::PathBuf)> {
    let mut found = Vec::new();
    let mut folders = vec![standard.to_path_buf()];
    while let Some(folder) = folders.pop() {
        for entry in std::fs::read_dir(&folder)
            .expect("a folder of the fixtures is read")
            .map(|entry| entry.expect("an entry of a folder of the fixtures is read"))
        {
            let path = entry.path();
            if path.is_dir() {
                folders.push(path);
                continue;
            }
            if path.extension().is_some_and(|each| each == "tree")
                && let Ok(below) = path.strip_prefix(standard)
            {
                let input = if below
                    .parent()
                    .is_some_and(|each| each.as_os_str().is_empty())
                {
                    std::path::Path::new("relations")
                        .join(below.file_stem().unwrap_or_default())
                        .join("input.css")
                } else {
                    below.with_extension("css")
                };
                if fixtures.join(&input).exists() {
                    found.push((below.to_path_buf(), input));
                }
            }
        }
    }
    found.sort();
    found
}

/// Whether our tree of `text` is the standard's `print`.
fn agrees(text: &str, print: &str) -> bool {
    match (projected(text), dumped(print)) {
        (Some(ours), Some(theirs)) => differences(&ours, &theirs, "").is_none(),
        _ => false,
    }
}

fn subject(given: Input) -> Result<Value, Failure> {
    match given {
        Given::Tree(text) => {
            check(&SyntaxCssparserAdapter);
            let shown = elements_of(&SyntaxCssparserAdapter, text)
                .iter()
                .map(|(kind, _)| kind_name(*kind).0)
                .collect::<Vec<_>>()
                .join(" ");
            Ok(Seen::Elements(Box::leak(shown.into_boxed_str())))
        }
        Given::Standard(text, print) => {
            let found = match (projected(text), dumped(print)) {
                (Some(ours), Some(theirs)) => differences(&ours, &theirs, ""),
                (None, _) => Some("our tree has no root".to_owned()),
                (_, None) => Some("the print does not read back".to_owned()),
            };
            Ok(Seen::Standard(
                found.map(|each| &*Box::leak(each.into_boxed_str())),
            ))
        }
        Given::Pinned => {
            let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
            let found = pinned(&fixtures.join("standard"), &fixtures);
            if found.is_empty() {
                return Err(());
            }
            let mut differing = Vec::new();
            for (print, input) in found {
                let (Ok(text), Ok(printed)) = (
                    std::fs::read_to_string(fixtures.join(&input)),
                    std::fs::read_to_string(fixtures.join("standard").join(&print)),
                ) else {
                    return Err(());
                };
                if !agrees(&text, &printed) {
                    differing.push(&*Box::leak(
                        input
                            .components()
                            .map(|part| part.as_os_str().to_string_lossy())
                            .collect::<Vec<_>>()
                            .join("/")
                            .into_boxed_str(),
                    ));
                }
            }
            Ok(Seen::Pinned(Box::leak(differing.into_boxed_slice())))
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
