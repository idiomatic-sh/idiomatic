//! The `ruff_python_parser` adapter runs the port's contract, and its own cases: the tokens spell the file, the strings carry what
//! the library decoded, and the tree is the library's.

use super::adapter::SyntaxRuffPythonParserAdapter;
use super::fixtures::diagnostics::diagnostics_of;
use super::fixtures::pinned::pinned;
use super::fixtures::standard::{differences, dumped, projected};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use crate::outbound::syntax::fixtures::contract::{check, elements_of};
use crate::outbound::syntax::interface::SyntaxPort as _;
use crate::outbound::syntax::types::{PythonSourceKind, SourceText};
use crate::settings::{PythonParserSettings, PythonTargetVersion};
use ruff_python_ast::token::Token as RuffToken;
use ryo::languages::{self, ByteSlice, Elements, Symbols};

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
    /// Parser diagnostics under explicit source-kind and target-version settings.
    ConfiguredDiagnostics(&'static str, PythonParserSettings, PythonSourceKind),
}

type Input = Given;
/// The kinds of the table's elements in order, with the text each spans; or, for the round trip, the tokens' text joined, the text
/// decoded for each string token, and how many diagnostics.
type Value = Seen;
type Failure = ();

#[derive(Debug, PartialEq)]
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
    SyntaxRuffPythonParserAdapter {
        settings: PythonParserSettings::default(),
        ..Default::default()
    }
    .read(SourceText(bytes), PythonSourceKind::Python, &mut builder);
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
        name: CaseName("the configured Python 310 parser reports unsupported type syntax"),
        word: Word::Succeeds,
        input: || {
            Given::ConfiguredDiagnostics(
                "class C[T = int, U]: ...\n",
                PythonParserSettings::new(PythonTargetVersion::Py310),
                PythonSourceKind::Python,
            )
        },
        expect: Expect::Value(Seen::Diagnostics(&[
            (
                "Cannot set default type for a type parameter on Python 3.10 (syntax was added in Python 3.13)",
                10,
                15,
            ),
            (
                "Cannot use type parameter lists on Python 3.10 (syntax was added in Python 3.12)",
                7,
                19,
            ),
            (
                "non default type parameter `U` follows default type parameter",
                17,
                18,
            ),
        ])),
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
                        input
                            .components()
                            .map(|part| part.as_os_str().to_string_lossy())
                            .collect::<Vec<_>>()
                            .join("/")
                            .into_boxed_str(),
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
            SyntaxRuffPythonParserAdapter {
                settings: PythonParserSettings::default(),
                ..Default::default()
            }
            .read(
                SourceText(text.as_bytes()),
                PythonSourceKind::Python,
                &mut builder,
            );
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
        Given::ConfiguredDiagnostics(text, settings, source_kind) => Ok(Seen::Diagnostics(
            diagnostics_of(text, source_kind, settings),
        )),
        Given::Tree(text) => {
            let elements: Vec<(Kind, &'static str)> =
                elements_of(&SyntaxRuffPythonParserAdapter::default(), text)
                    .into_iter()
                    .map(|(kind, text)| (kind, &*Box::leak(text.into_boxed_str())))
                    .collect();
            Ok(Seen::Elements(Box::leak(elements.into_boxed_slice())))
        }
    }
}

fn preserves_async_statement_kinds() {
    let source = "async def f(items):\n    async for item in items:\n        async with manager():\n            pass\n    def g():\n        for item in items:\n            with manager():\n                pass\n";
    let seen = elements_of(&SyntaxRuffPythonParserAdapter::default(), source);
    let kinds: Vec<_> = seen.into_iter().map(|(kind, _)| kind).collect();

    assert!(kinds.contains(&Kind::Node(Node::AsyncFunctionDef)));
    assert!(kinds.contains(&Kind::Node(Node::AsyncFor)));
    assert!(kinds.contains(&Kind::Node(Node::AsyncWith)));
    assert!(kinds.contains(&Kind::Node(Node::FunctionDef)));
    assert!(kinds.contains(&Kind::Node(Node::For)));
    assert!(kinds.contains(&Kind::Node(Node::With)));
}

/// Which of the file's cases a row runs.
#[derive(Clone, Copy, Debug)]
enum Body {
    /// The case `preserves_async_statement_kinds`.
    PreservesAsyncStatementKinds,
}

/// The rows of the cases whose bodies assert for themselves.
const BODIES: &[Case<Body, (), ()>] = &[Case {
    name: CaseName("preserves_async_statement_kinds"),
    word: Word::Succeeds,
    input: || Body::PreservesAsyncStatementKinds,
    expect: Expect::Value(()),
}];

/// Runs the body of one case.
fn body(given: Body) -> Result<(), ()> {
    match given {
        Body::PreservesAsyncStatementKinds => preserves_async_statement_kinds(),
    }
    Ok(())
}

#[test]
fn succeeds() {
    check(&SyntaxRuffPythonParserAdapter::default());
    run(CASES, Word::Succeeds, subject);
    run(BODIES, Word::Succeeds, body);
}
