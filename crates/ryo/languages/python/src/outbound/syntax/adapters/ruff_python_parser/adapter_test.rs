//! The `ruff_python_parser` adapter runs the port's contract, and its own cases: the tokens spell the file, the strings carry what
//! the library decoded, and the tree is the library's.

use super::adapter::SyntaxRuffPythonParserAdapter;
use super::fixtures::standard::{differences, dumped, projected};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use crate::outbound::syntax::fixtures::contract::{check, elements_of};
use crate::outbound::syntax::interface::SyntaxPort as _;
use crate::outbound::syntax::types::SourceText;
use ryo::languages::{self, ByteSlice, Elements, Symbols};

/// What a row reads: a file whose tokens are joined again, or a file whose elements are listed.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The round trip of the bytes.
    Read(&'static [u8]),
    /// The elements of the text.
    Tree(&'static str),
    /// A file and what `CPython`'s `ast.dump` printed for it: our tree projected onto the standard has the same nodes, each in the same
    /// field.
    Standard(&'static str, &'static str),
    /// Every relation's input, and every file of the corpus and of the suite's valid half, whose `ast.dump` is pinned in `standard/`.
    Pinned,
    /// A file `CPython`'s parser refuses, and its refusal, `line:column: message`: the language reports the same, on the same line.
    Refused(&'static str, &'static str),
    /// The features `from __future__ import` takes, as `CPython`'s `__future__.all_feature_names`, pinned, lists them.
    Features(&'static str),
    /// The semantic diagnostics the parser adapter preserves, with their byte ranges.
    Diagnostics(&'static str),
}

type Input = Given;
/// The kinds of the table's elements in order, with the text each spans; or, for the round trip, the tokens' text joined, the text
/// decoded for each string token, and how many diagnostics.
type Value = Seen;
type Failure = ();

/// What a case looks at.
#[derive(Debug, PartialEq, Eq)]
enum Seen {
    /// Every element: its kind and the text it spans.
    Elements(&'static [(Kind, &'static str)]),
    /// What differs between our tree projected onto the standard and `CPython`'s.
    Standard(Option<&'static str>),
    /// The pinned files whose tree differs from `CPython`'s, by path below `tests/fixtures`.
    Pinned(&'static [&'static str]),
    /// Whether the language reports `CPython`'s refusal: its message, on its line.
    Refused(bool),
    /// Whether the adapter's features are the pinned ones, in order.
    Features(bool),
    /// The tokens' bytes joined, the decoded texts in order, and how many diagnostics.
    Read(&'static [u8], &'static [&'static str], usize),
    /// The parser's diagnostic messages and byte ranges, in report order.
    Diagnostics(&'static [(&'static str, u32, u32)]),
}

/// What the adapter reads `bytes` into, as the round trip sees it.
fn round_trip(bytes: &'static [u8]) -> Seen {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = languages::table_builder(&mut elements, &mut symbols, ByteSlice(bytes));
    SyntaxRuffPythonParserAdapter.read(SourceText(bytes), &mut builder);
    let mut joined = Vec::new();
    let mut decoded = Vec::new();
    for at in languages::positions(&elements) {
        if languages::flags_at(&elements, at).0 & languages::TOKEN.0 != 0 {
            let (start, end) = (
                usize::try_from(languages::start_at(&elements, at).0).unwrap_or(0),
                usize::try_from(languages::end_at(&elements, at).0).unwrap_or(0),
            );
            joined.extend_from_slice(bytes.get(start..end).unwrap_or_default());
            if let Some(text) = languages::decoded_at(&elements, at) {
                decoded.push(text.0.to_owned());
            }
        }
    }
    let decoded: Vec<&'static str> = decoded
        .into_iter()
        .map(|each| &*Box::leak(each.into_boxed_str()))
        .collect();
    Seen::Read(
        Box::leak(joined.into_boxed_slice()),
        Box::leak(decoded.into_boxed_slice()),
        languages::diagnostic_count(&elements).0,
    )
}

/// The rows of this part's cases: the tokens give back every byte, the strings carry their decoded text, and the tree is the
/// library's.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_byte_order_mark_crlf_and_multibyte_text_round_trip"),
        word: Word::Succeeds,
        input: || Given::Read("\u{feff}# π\r\nvalue = 'é\\n'\r\n".as_bytes()),
        expect: Expect::Value(Seen::Read(
            "\u{feff}# π\r\nvalue = 'é\\n'\r\n".as_bytes(),
            &["é\n"],
            0,
        )),
    },
    Case {
        name: CaseName("bytes_that_are_not_utf8_are_read_with_a_diagnostic"),
        word: Word::Succeeds,
        input: || Given::Read(b"x = 1\n\xff\xfe\n"),
        expect: Expect::Value(Seen::Read(b"x = 1\n\xff\xfe\n", &[], 5)),
    },
    Case {
        name: CaseName("the_literal_parts_of_an_f_string_carry_their_decoded_text"),
        word: Word::Succeeds,
        input: || Given::Read(b"f'a{{b}}{c}'\n"),
        expect: Expect::Value(Seen::Read(b"f'a{{b}}{c}'\n", &["a{b}"], 0)),
    },
    Case {
        name: CaseName("an_empty_file_round_trips"),
        word: Word::Succeeds,
        input: || Given::Read(b""),
        expect: Expect::Value(Seen::Read(b"", &[], 0)),
    },
    Case {
        name: CaseName("a_module_nonlocal_has_the_pinned_semantic_syntax_error"),
        word: Word::Succeeds,
        input: || Given::Diagnostics("nonlocal x\n"),
        expect: Expect::Value(Seen::Diagnostics(&[(
            "nonlocal declaration not allowed at module level",
            0,
            10,
        )])),
    },
    Case {
        name: CaseName("a_nonlocal_after_global_has_the_pinned_statement_range"),
        word: Word::Succeeds,
        input: || Given::Diagnostics("global x\nnonlocal x\n"),
        expect: Expect::Value(Seen::Diagnostics(&[(
            "nonlocal declaration not allowed at module level",
            9,
            19,
        )])),
    },
    Case {
        name: CaseName("a_nonlocal_in_a_module_compound_statement_is_still_module_scoped"),
        word: Word::Succeeds,
        input: || Given::Diagnostics("if True:\n    nonlocal x\n"),
        expect: Expect::Value(Seen::Diagnostics(&[(
            "nonlocal declaration not allowed at module level",
            13,
            23,
        )])),
    },
    Case {
        name: CaseName("a_nonlocal_in_a_module_loop_is_still_module_scoped"),
        word: Word::Succeeds,
        input: || Given::Diagnostics("for x in xs:\n    nonlocal x\n"),
        expect: Expect::Value(Seen::Diagnostics(&[(
            "nonlocal declaration not allowed at module level",
            17,
            27,
        )])),
    },
    Case {
        name: CaseName("a_nested_nonlocal_does_not_get_the_module_scope_diagnostic"),
        word: Word::Succeeds,
        input: || Given::Diagnostics("def f():\n    nonlocal x\n"),
        expect: Expect::Value(Seen::Diagnostics(&[])),
    },
    Case {
        name: CaseName("a_top_level_class_nonlocal_gets_no_module_scope_diagnostic"),
        word: Word::Succeeds,
        input: || Given::Diagnostics("class C:\n    nonlocal x\n"),
        expect: Expect::Value(Seen::Diagnostics(&[])),
    },
    Case {
        name: CaseName("a_class_inside_a_function_can_use_an_enclosing_nonlocal"),
        word: Word::Succeeds,
        input: || Given::Diagnostics("def outer():\n    x = 1\n    class C:\n        nonlocal x\n"),
        expect: Expect::Value(Seen::Diagnostics(&[])),
    },
    Case {
        name: CaseName("duplicate_function_parameters_report_each_repeated_name"),
        word: Word::Succeeds,
        input: || Given::Diagnostics("def f(a, /, a, *, b, b): ...\n"),
        expect: Expect::Value(Seen::Diagnostics(&[
            ("Duplicate parameter \"a\"", 12, 13),
            ("Duplicate parameter \"b\"", 21, 22),
        ])),
    },
    Case {
        name: CaseName("positional_only_keyword_only_and_variadic_parameters_can_be_distinct"),
        word: Word::Succeeds,
        input: || Given::Diagnostics("def f(a, /, b, *args, c, **kwargs): ...\n"),
        expect: Expect::Value(Seen::Diagnostics(&[])),
    },
    Case {
        name: CaseName("duplicate_lambda_parameters_are_reported"),
        word: Word::Succeeds,
        input: || Given::Diagnostics("lambda x, x: x\n"),
        expect: Expect::Value(Seen::Diagnostics(&[("Duplicate parameter \"x\"", 10, 11)])),
    },
    Case {
        name: CaseName("same_parameter_name_in_nested_functions_is_not_a_duplicate"),
        word: Word::Succeeds,
        input: || Given::Diagnostics("def outer(x):\n    def inner(x): ...\n"),
        expect: Expect::Value(Seen::Diagnostics(&[])),
    },
    Case {
        name: CaseName("duplicate_type_parameters_report_each_later_occurrence"),
        word: Word::Succeeds,
        input: || Given::Diagnostics("def f[T, T, T](): ...\n"),
        expect: Expect::Value(Seen::Diagnostics(&[
            ("duplicate type parameter", 9, 10),
            ("duplicate type parameter", 12, 13),
        ])),
    },
    Case {
        name: CaseName("non_default_type_parameters_after_a_default_are_reported"),
        word: Word::Succeeds,
        input: || Given::Diagnostics("def f[T = int, U, V](): ...\n"),
        expect: Expect::Value(Seen::Diagnostics(&[
            (
                "non default type parameter `U` follows default type parameter",
                15,
                16,
            ),
            (
                "non default type parameter `V` follows default type parameter",
                18,
                19,
            ),
        ])),
    },
    Case {
        name: CaseName("a_fully_defaulted_type_parameter_suffix_is_valid"),
        word: Word::Succeeds,
        input: || Given::Diagnostics("def f[T, U = int, V = str](): ...\n"),
        expect: Expect::Value(Seen::Diagnostics(&[])),
    },
    Case {
        name: CaseName("class_type_parameter_default_order_uses_its_local_scope"),
        word: Word::Succeeds,
        input: || Given::Diagnostics("class C[T = int, U]: ...\n"),
        expect: Expect::Value(Seen::Diagnostics(&[(
            "non default type parameter `U` follows default type parameter",
            17,
            18,
        )])),
    },
    Case {
        name: CaseName("duplicate_call_keywords_report_the_second_keyword_range"),
        word: Word::Succeeds,
        input: || Given::Diagnostics("call(x=1, x=2)\n"),
        expect: Expect::Value(Seen::Diagnostics(&[(
            "Duplicate keyword argument `x`",
            10,
            13,
        )])),
    },
    Case {
        name: CaseName("dictionary_expansion_keyword_duplicates_are_not_syntax_errors"),
        word: Word::Succeeds,
        input: || Given::Diagnostics("call(x=1, **{\"x\": 2})\n"),
        expect: Expect::Value(Seen::Diagnostics(&[])),
    },
    Case {
        name: CaseName("duplicate_class_keywords_report_the_second_keyword_range"),
        word: Word::Succeeds,
        input: || Given::Diagnostics("class C(metaclass=A, metaclass=B): ...\n"),
        expect: Expect::Value(Seen::Diagnostics(&[(
            "Duplicate keyword argument `metaclass`",
            21,
            32,
        )])),
    },
    Case {
        name: CaseName("duplicate_parameter_diagnostic_survives_an_unbound_name_in_the_body"),
        word: Word::Succeeds,
        input: || Given::Diagnostics("def f(x, x):\n    print(missing)\n"),
        expect: Expect::Value(Seen::Diagnostics(&[("Duplicate parameter \"x\"", 9, 10)])),
    },
    Case {
        name: CaseName("a_call_is_nodes_with_its_punctuation_as_tokens"),
        word: Word::Succeeds,
        input: || Given::Tree("f(x)"),
        expect: Expect::Value(Seen::Elements(&[
            (Kind::Node(Node::Module), "f(x)"),
            (Kind::Node(Node::Expr), "f(x)"),
            (Kind::Node(Node::Call), "f(x)"),
            (Kind::Node(Node::Name), "f"),
            (Kind::Token(Token::Name), "f"),
            (Kind::Extra(Extra::CallArguments), "(x)"),
            (Kind::Token(Token::Lpar), "("),
            (Kind::Node(Node::Name), "x"),
            (Kind::Token(Token::Name), "x"),
            (Kind::Token(Token::Rpar), ")"),
            (Kind::Token(Token::Newline), ""),
        ])),
    },
    Case {
        name: CaseName("a_tree_that_is_not_cpython_s_is_a_difference"),
        word: Word::Succeeds,
        input: || Given::Standard("x = 1\n", "Module(body=[Pass()])"),
        expect: Expect::Value(Seen::Standard(Some(
            "/module: we have [\"body=assign(targets=name() value=constant())\"], CPython has [\"body=pass()\"]",
        ))),
    },
    Case {
        name: CaseName("a_repeated_child_the_standard_has_once_is_a_difference"),
        word: Word::Succeeds,
        input: || {
            Given::Standard(
                "pass\nraise\nraise\n",
                "Module(body=[Pass(), Pass(), Raise()])",
            )
        },
        expect: Expect::Value(Seen::Standard(Some(
            "/module: we have [\"body=raise()\"], CPython has [\"body=pass()\"]",
        ))),
    },
    Case {
        name: CaseName("a_node_in_another_field_than_the_standard_s_is_a_difference"),
        word: Word::Succeeds,
        input: || {
            Given::Standard(
                "f(x)\n",
                "Module(body=[Expr(value=Call(func=Name(id='f'), keywords=[Name(id='x')]))])",
            )
        },
        expect: Expect::Value(Seen::Standard(Some(
            "/module/expr/call: we have [\"args=name()\"], CPython has [\"keywords=name()\"]",
        ))),
    },
    Case {
        name: CaseName("every_pinned_relation_corpus_and_suite_file_has_cpython_s_nodes_but_one"),
        word: Word::Succeeds,
        input: || Given::Pinned,
        // The one exception: `case {+1: ...}`. CPython 3.15 folds the key `+1` of a mapping pattern into a `Constant`; ruff keeps the
        // `UnaryOp` it is written as, so the tree has one node more than the standard's there.
        expect: Expect::Value(Seen::Pinned(&["suite/valid/inline/unary_plus_py315.py"])),
    },
    Case {
        name: CaseName("an_else_with_no_except_is_refused_as_cpython_refuses_it"),
        word: Word::Succeeds,
        input: || {
            Given::Refused(
                include_str!(
                    "../../../../../tests/fixtures/standard/refused/else_without_except.py"
                ),
                include_str!(
                    "../../../../../tests/fixtures/standard/refused/else_without_except.refused"
                ),
            )
        },
        expect: Expect::Value(Seen::Refused(true)),
    },
    Case {
        name: CaseName("an_unknown_future_feature_is_refused_as_cpython_refuses_it"),
        word: Word::Succeeds,
        input: || {
            Given::Refused(
                include_str!("../../../../../tests/fixtures/standard/refused/unknown_future.py"),
                include_str!(
                    "../../../../../tests/fixtures/standard/refused/unknown_future.refused"
                ),
            )
        },
        expect: Expect::Value(Seen::Refused(true)),
    },
    Case {
        name: CaseName("the_future_features_are_cpython_s"),
        word: Word::Succeeds,
        input: || {
            Given::Features(include_str!(
                "../../../../../tests/fixtures/standard/future_features.txt"
            ))
        },
        expect: Expect::Value(Seen::Features(true)),
    },
];

/// The `ast.dump` outputs pinned below `standard`, each with the file it was printed for, both by path below `fixtures`, sorted: for the
/// corpus and the suite at the same path, and for each relation's input beside the page `standard/<relation>.ast`.
fn pinned(
    standard: &std::path::Path,
    fixtures: &std::path::Path,
) -> Vec<(std::path::PathBuf, std::path::PathBuf)> {
    let mut found = Vec::new();
    let mut folders = vec![standard.join("corpus"), standard.join("suite")];
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
            if path.extension().is_some_and(|each| each == "ast")
                && let Ok(below) = path.strip_prefix(standard)
                && fixtures.join(below).with_extension("py").exists()
            {
                found.push((below.to_path_buf(), below.with_extension("py")));
            }
        }
    }
    for entry in std::fs::read_dir(standard)
        .expect("a folder of the fixtures is read")
        .map(|entry| entry.expect("an entry of a folder of the fixtures is read"))
    {
        let path = entry.path();
        let input = std::path::Path::new("relations")
            .join(path.file_stem().unwrap_or_default())
            .join("input.py");
        if path.extension().is_some_and(|each| each == "ast") && fixtures.join(&input).exists() {
            found.push((
                path.strip_prefix(standard).unwrap_or(&path).to_path_buf(),
                input,
            ));
        }
    }
    found.sort();
    found
}

fn subject(given: Input) -> Result<Value, Failure> {
    match given {
        Given::Read(bytes) => Ok(round_trip(bytes)),
        Given::Standard(text, dump) => {
            let found = match (projected(text), dumped(dump)) {
                (Some(ours), Some(theirs)) => differences(&ours, &theirs, ""),
                (None, _) => Some("our tree has no root".to_owned()),
                (_, None) => Some("the dump does not read back".to_owned()),
            };
            Ok(Seen::Standard(
                found.map(|each| &*Box::leak(each.into_boxed_str())),
            ))
        }
        Given::Pinned => {
            let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
            let mut differing = Vec::new();
            for (dump, input) in pinned(&fixtures.join("standard"), &fixtures) {
                let source = fixtures.join(&input);
                let (Ok(text), Ok(printed)) = (
                    std::fs::read_to_string(&source),
                    std::fs::read_to_string(fixtures.join("standard").join(&dump)),
                ) else {
                    return Err(());
                };
                let agrees = match (projected(&text), dumped(&printed)) {
                    (Some(ours), Some(theirs)) => differences(&ours, &theirs, "").is_none(),
                    _ => false,
                };
                if !agrees {
                    differing.push(&*Box::leak(
                        input.to_string_lossy().into_owned().into_boxed_str(),
                    ));
                }
            }
            if differing.is_empty() && pinned(&fixtures.join("standard"), &fixtures).is_empty() {
                return Err(());
            }
            Ok(Seen::Pinned(Box::leak(differing.into_boxed_slice())))
        }
        Given::Refused(text, refusal) => {
            let (place, message) = refusal.trim_end().split_once(": ").ok_or(())?;
            let line: usize = place
                .split(':')
                .next()
                .and_then(|each| each.parse().ok())
                .ok_or(())?;
            let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
            let mut builder =
                languages::table_builder(&mut elements, &mut symbols, ByteSlice(text.as_bytes()));
            SyntaxRuffPythonParserAdapter.read(SourceText(text.as_bytes()), &mut builder);
            let reported = languages::diagnostics(&elements).any(|each| {
                let start = usize::try_from(each.start.0).unwrap_or(usize::MAX);
                let on = text
                    .get(..start)
                    .map_or(0, |before| before.matches('\n').count() + 1);
                each.message.0 == message.as_bytes() && on == line
            });
            Ok(Seen::Refused(reported))
        }
        Given::Features(pinned) => Ok(Seen::Features(
            pinned
                .lines()
                .eq(super::constants::FUTURE_FEATURES.iter().copied()),
        )),
        Given::Diagnostics(text) => {
            let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
            let mut builder =
                languages::table_builder(&mut elements, &mut symbols, ByteSlice(text.as_bytes()));
            SyntaxRuffPythonParserAdapter.read(SourceText(text.as_bytes()), &mut builder);
            let diagnostics: Vec<(&'static str, u32, u32)> = languages::diagnostics(&elements)
                .map(|diagnostic| {
                    let message = std::str::from_utf8(diagnostic.message.0).unwrap_or_default();
                    (
                        &*Box::leak(message.to_owned().into_boxed_str()),
                        diagnostic.start.0,
                        diagnostic.end.0,
                    )
                })
                .collect();
            Ok(Seen::Diagnostics(Box::leak(diagnostics.into_boxed_slice())))
        }
        Given::Tree(text) => {
            let elements: Vec<(Kind, &'static str)> =
                elements_of(&SyntaxRuffPythonParserAdapter, text)
                    .into_iter()
                    .map(|(kind, text)| (kind, &*Box::leak(text.into_boxed_str())))
                    .collect();
            Ok(Seen::Elements(Box::leak(elements.into_boxed_slice())))
        }
    }
}

#[test]
fn succeeds() {
    check(&SyntaxRuffPythonParserAdapter);
    run(CASES, Word::Succeeds, subject);
}
