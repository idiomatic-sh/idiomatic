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
use crate::outbound::syntax::types::{PythonSourceKind, SourceText};
use crate::settings::{PythonParserSettings, PythonTargetVersion};
use ruff_python_ast::{PySourceType, PythonVersion};
use ryo::languages::{self, ByteSlice, Elements, Symbols, TreeImpact};

#[test]
fn preserves_async_statement_kinds() {
    let source = "async def f(items):\n    async for item in items:\n        async with manager():\n            pass\n    def g():\n        for item in items:\n            with manager():\n                pass\n";
    let seen = elements_of(&SyntaxRuffPythonParserAdapter, source);
    let kinds: Vec<_> = seen.into_iter().map(|(kind, _)| kind).collect();

    assert!(kinds.contains(&Kind::Node(Node::AsyncFunctionDef)));
    assert!(kinds.contains(&Kind::Node(Node::AsyncFor)));
    assert!(kinds.contains(&Kind::Node(Node::AsyncWith)));
    assert!(kinds.contains(&Kind::Node(Node::FunctionDef)));
    assert!(kinds.contains(&Kind::Node(Node::For)));
    assert!(kinds.contains(&Kind::Node(Node::With)));
}

#[test]
fn target_version_syntax_warning_preserves_the_tree() {
    let source = "try:\n    pass\nexcept* (ValueError,):\n    pass\n";
    let diagnostics_for = |target| {
        let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
        let mut builder =
            languages::table_builder(&mut elements, &mut symbols, ByteSlice(source.as_bytes()));
        SyntaxRuffPythonParserAdapter.read(
            SourceText(source.as_bytes()),
            PythonSourceKind::Python,
            &PythonParserSettings::new(target),
            &mut builder,
        );
        let diagnostics: Vec<_> = languages::diagnostics(&elements)
            .map(|diagnostic| {
                (
                    diagnostic.phase.name().to_owned(),
                    diagnostic.impact.name().to_owned(),
                    diagnostic.code.as_str().to_owned(),
                    diagnostic.start.0,
                    diagnostic.end.0,
                )
            })
            .collect();
        let acceptance = languages::syntax_acceptance(&elements);
        (elements, diagnostics, acceptance)
    };

    let (py310, unsupported, py310_acceptance) = diagnostics_for(PythonTargetVersion::Py310);
    assert_eq!(py310_acceptance, languages::SyntaxAcceptance::Accepted);
    assert!(!languages::has_incomplete_syntax(&py310));
    assert_eq!(unsupported.len(), 1);
    assert_eq!(unsupported[0].0, "validation");
    assert_eq!(unsupported[0].1, "preserved");
    assert_eq!(unsupported[0].2, "unsupported-syntax");
    assert_eq!((unsupported[0].3, unsupported[0].4), (20, 21));

    let (_py311, supported, py311_acceptance) = diagnostics_for(PythonTargetVersion::Py311);
    assert!(supported.is_empty());
    assert_eq!(py311_acceptance, languages::SyntaxAcceptance::Accepted);
}

#[test]
fn semantic_syntax_checks_only_run_after_an_accepted_parse() {
    let diagnostics_for = |source: &str| {
        let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
        let mut builder =
            languages::table_builder(&mut elements, &mut symbols, ByteSlice(source.as_bytes()));
        SyntaxRuffPythonParserAdapter.read(
            SourceText(source.as_bytes()),
            PythonSourceKind::Python,
            &PythonParserSettings::new(PythonTargetVersion::Py310),
            &mut builder,
        );
        let diagnostics: Vec<_> = languages::diagnostics(&elements)
            .map(|diagnostic| {
                (
                    diagnostic.phase.name().to_owned(),
                    diagnostic.code.as_str().to_owned(),
                )
            })
            .collect();
        (languages::syntax_acceptance(&elements), diagnostics)
    };

    let (accepted, semantic_errors) = diagnostics_for("nonlocal x\n");
    assert_eq!(accepted, languages::SyntaxAcceptance::Accepted);
    assert!(
        semantic_errors
            .iter()
            .any(|(phase, code)| { phase == "validation" && code == "nonlocal-at-module" })
    );

    let (rejected, parse_errors) = diagnostics_for("nonlocal x\nx =\n");
    assert_eq!(rejected, languages::SyntaxAcceptance::Rejected);
    assert!(
        parse_errors
            .iter()
            .any(|(phase, code)| { phase == "parse" && code == "parser-error" })
    );
    assert!(
        !parse_errors
            .iter()
            .any(|(phase, code)| { phase == "validation" && code == "nonlocal-at-module" })
    );
}

#[test]
fn named_expression_in_stub_annotation_is_a_parse_diagnostic_only_for_stubs() {
    let diagnostics_for = |source: &str, source_kind| {
        let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
        let mut builder =
            languages::table_builder(&mut elements, &mut symbols, ByteSlice(source.as_bytes()));
        SyntaxRuffPythonParserAdapter.read(
            SourceText(source.as_bytes()),
            source_kind,
            &PythonParserSettings::new(PythonTargetVersion::Py310),
            &mut builder,
        );
        let diagnostics: Vec<_> = languages::diagnostics(&elements)
            .map(|diagnostic| {
                (
                    diagnostic.phase.name().to_owned(),
                    diagnostic.impact.name().to_owned(),
                    diagnostic.code.as_str().to_owned(),
                    diagnostic.start.0,
                    diagnostic.end.0,
                )
            })
            .collect();
        (
            languages::syntax_acceptance(&elements),
            languages::has_incomplete_syntax(&elements),
            diagnostics,
        )
    };

    let source = "def f(x: (a := b)): ...\n";
    let (stub_acceptance, stub_incomplete, stub_diagnostics) =
        diagnostics_for(source, PythonSourceKind::Stub);
    assert_eq!(stub_acceptance, languages::SyntaxAcceptance::Accepted);
    assert!(
        !stub_incomplete,
        "semantic syntax diagnostics preserve AST facts"
    );
    assert_eq!(
        stub_diagnostics,
        [(
            "parse".to_owned(),
            "preserved".to_owned(),
            "parser-error".to_owned(),
            10,
            16
        )]
    );

    let (python_acceptance, python_incomplete, python_diagnostics) =
        diagnostics_for(source, PythonSourceKind::Python);
    assert_eq!(python_acceptance, languages::SyntaxAcceptance::Accepted);
    assert!(!python_incomplete);
    assert!(python_diagnostics.is_empty());

    let future_source = "from __future__ import annotations\ndef f(x: (a := b)): ...\n";
    let (future_acceptance, future_incomplete, future_diagnostics) =
        diagnostics_for(future_source, PythonSourceKind::Python);
    assert_eq!(future_acceptance, languages::SyntaxAcceptance::Accepted);
    assert!(!future_incomplete);
    assert_eq!(
        future_diagnostics,
        [(
            "parse".to_owned(),
            "preserved".to_owned(),
            "parser-error".to_owned(),
            45,
            51
        )]
    );
}

#[test]
fn parser_errors_distinguish_complete_tokens_from_lexical_failure() {
    let impacts = |source: &str| {
        let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
        let mut builder =
            languages::table_builder(&mut elements, &mut symbols, ByteSlice(source.as_bytes()));
        SyntaxRuffPythonParserAdapter.read(
            SourceText(source.as_bytes()),
            PythonSourceKind::Python,
            &PythonParserSettings::new(PythonTargetVersion::Py310),
            &mut builder,
        );
        let impacts: Vec<_> = languages::diagnostics(&elements)
            .map(|diagnostic| diagnostic.impact)
            .collect();
        (
            impacts,
            languages::has_incomplete_syntax(&elements),
            languages::has_incomplete_lexical_stream(&elements),
            languages::syntax_acceptance(&elements),
        )
    };

    for prefix in ["ur", "ru", "UR", "bu"] {
        let source = format!("value = {prefix}\"text\" # retained comment\n");
        let (impacts, incomplete_syntax, incomplete_lexical, acceptance) = impacts(&source);
        assert!(!impacts.is_empty(), "{prefix} must be rejected for PY310");
        assert!(
            impacts
                .iter()
                .all(|impact| *impact == TreeImpact::LexicallyComplete),
            "{prefix}: {impacts:?}"
        );
        assert!(incomplete_syntax);
        assert!(!incomplete_lexical);
        assert_eq!(acceptance, languages::SyntaxAcceptance::Rejected);
    }

    for source in [
        "value = 'unterminated\n",
        "if ready:\n    work(\"unfinished\n",
    ] {
        let (impacts, incomplete_syntax, incomplete_lexical, acceptance) = impacts(source);
        assert!(!impacts.is_empty(), "source must report a parse error");
        assert!(incomplete_syntax);
        assert!(
            incomplete_lexical,
            "{source:?} must not certify lexical facts"
        );
        assert_eq!(acceptance, languages::SyntaxAcceptance::Rejected);
    }
}

#[test]
fn parser_refusal_with_complete_tokens_keeps_lexical_facts_available() {
    let source = "try:\n    work()\nelse:\n    recover()\n";
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder =
        languages::table_builder(&mut elements, &mut symbols, ByteSlice(source.as_bytes()));
    SyntaxRuffPythonParserAdapter.read(
        SourceText(source.as_bytes()),
        PythonSourceKind::Python,
        &PythonParserSettings::default(),
        &mut builder,
    );
    let diagnostics: Vec<_> = languages::diagnostics(&elements).collect();
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == "else-without-except")
    );
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.phase == languages::DiagnosticPhase::Parse
            && diagnostic.impact == TreeImpact::LexicallyComplete
    }));
    assert!(languages::has_incomplete_syntax(&elements));
    assert!(!languages::has_incomplete_lexical_stream(&elements));
}

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
    /// Parser diagnostics under explicit source-kind and target-version settings.
    ConfiguredDiagnostics(&'static str, PythonParserSettings, PythonSourceKind),
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
    SyntaxRuffPythonParserAdapter.read(
        SourceText(bytes),
        PythonSourceKind::Python,
        &PythonParserSettings::default(),
        &mut builder,
    );
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
        name: CaseName("global_parameter_errors_follow_the_declaration_name"),
        word: Word::Succeeds,
        input: || Given::Diagnostics("def f(x):\n    global x\n"),
        expect: Expect::Value(Seen::Diagnostics(&[(
            "name `x` cannot refer to a parameter and a global variable",
            21,
            22,
        )])),
    },
    Case {
        name: CaseName("nonlocal_parameter_errors_even_with_an_outer_binding"),
        word: Word::Succeeds,
        input: || {
            Given::Diagnostics("def outer():\n    x = 1\n    def inner(x):\n        nonlocal x\n")
        },
        expect: Expect::Value(Seen::Diagnostics(&[(
            "name `x` cannot refer to a parameter and a nonlocal variable",
            58,
            59,
        )])),
    },
    Case {
        name: CaseName("each_repeated_global_parameter_declaration_is_reported"),
        word: Word::Succeeds,
        input: || Given::Diagnostics("def f(x):\n    global x\n    global x\n"),
        expect: Expect::Value(Seen::Diagnostics(&[
            (
                "name `x` cannot refer to a parameter and a global variable",
                21,
                22,
            ),
            (
                "name `x` cannot refer to a parameter and a global variable",
                34,
                35,
            ),
        ])),
    },
    Case {
        name: CaseName("global_and_annotation_diagnostics_follow_source_order"),
        word: Word::Succeeds,
        input: || Given::Diagnostics("def f(x):\n    x: int\n    global x\n    x: str\n"),
        expect: Expect::Value(Seen::Diagnostics(&[
            ("annotated name `x` can't be global", 14, 15),
            (
                "name `x` cannot refer to a parameter and a global variable",
                32,
                33,
            ),
            ("annotated name `x` can't be global", 38, 39),
        ])),
    },
    Case {
        name: CaseName("a_module_annotation_before_repeated_globals_is_reported_once"),
        word: Word::Succeeds,
        input: || Given::Diagnostics("x: int\nglobal x\nglobal x\n"),
        expect: Expect::Value(Seen::Diagnostics(&[(
            "annotated name `x` can't be global",
            0,
            1,
        )])),
    },
    Case {
        name: CaseName("a_module_annotation_after_global_is_allowed"),
        word: Word::Succeeds,
        input: || Given::Diagnostics("global x\nx: int\n"),
        expect: Expect::Value(Seen::Diagnostics(&[])),
    },
    Case {
        name: CaseName("a_later_module_global_makes_an_intervening_annotation_invalid"),
        word: Word::Succeeds,
        input: || Given::Diagnostics("global x\nx: int\nglobal x\n"),
        expect: Expect::Value(Seen::Diagnostics(&[(
            "annotated name `x` can't be global",
            9,
            10,
        )])),
    },
    Case {
        name: CaseName("nested_scope_declarations_do_not_leak_across_function_class_or_lambda"),
        word: Word::Succeeds,
        input: || {
            Given::Diagnostics(
                "def outer(x):\n    global x\n    class C:\n        y: int\n        def inner(y):\n            global y\n    callback = lambda x: x\n",
            )
        },
        expect: Expect::Value(Seen::Diagnostics(&[
            (
                "name `x` cannot refer to a parameter and a global variable",
                25,
                26,
            ),
            (
                "name `y` cannot refer to a parameter and a global variable",
                96,
                97,
            ),
        ])),
    },
    Case {
        name: CaseName("a_class_scope_global_annotation_is_not_the_enclosing_function_global"),
        word: Word::Succeeds,
        input: || Given::Diagnostics("def f(x):\n    class C:\n        global x\n        x: int\n"),
        expect: Expect::Value(Seen::Diagnostics(&[(
            "annotated name `x` can't be global",
            48,
            49,
        )])),
    },
    Case {
        name: CaseName("unicode_global_parameter_ranges_are_utf8_bytes"),
        word: Word::Succeeds,
        input: || Given::Diagnostics("def f(π):\n    global π\n"),
        expect: Expect::Value(Seen::Diagnostics(&[(
            "name `π` cannot refer to a parameter and a global variable",
            22,
            24,
        )])),
    },
    Case {
        name: CaseName("unicode_annotated_global_ranges_are_utf8_bytes"),
        word: Word::Succeeds,
        input: || Given::Diagnostics("def f():\n    global π\n    π: int\n"),
        expect: Expect::Value(Seen::Diagnostics(&[(
            "annotated name `π` can't be global",
            27,
            29,
        )])),
    },
    Case {
        name: CaseName("module_nonlocal_is_reported_without_other_module_scope_errors"),
        word: Word::Succeeds,
        input: || Given::Diagnostics("global x\nnonlocal y\nx: int\n"),
        expect: Expect::Value(Seen::Diagnostics(&[(
            "nonlocal declaration not allowed at module level",
            9,
            19,
        )])),
    },
    Case {
        name: CaseName("global_indexes_are_local_to_the_function_scope"),
        word: Word::Succeeds,
        input: || Given::Diagnostics("global x\ndef f():\n    x: int\n"),
        expect: Expect::Value(Seen::Diagnostics(&[])),
    },
    Case {
        name: CaseName("a_nested_function_global_does_not_affect_its_parent_annotation"),
        word: Word::Succeeds,
        input: || {
            Given::Diagnostics("def outer():\n    def inner():\n        global x\n    x: int\n")
        },
        expect: Expect::Value(Seen::Diagnostics(&[])),
    },
    Case {
        name: CaseName("each_repeated_nonlocal_parameter_declaration_is_reported"),
        word: Word::Succeeds,
        input: || {
            Given::Diagnostics(
                "def outer():\n    x = 1\n    def inner(x):\n        nonlocal x\n        nonlocal x\n",
            )
        },
        expect: Expect::Value(Seen::Diagnostics(&[
            (
                "name `x` cannot refer to a parameter and a nonlocal variable",
                58,
                59,
            ),
            (
                "name `x` cannot refer to a parameter and a nonlocal variable",
                77,
                78,
            ),
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
            SyntaxRuffPythonParserAdapter.read(
                SourceText(text.as_bytes()),
                PythonSourceKind::Python,
                &PythonParserSettings::default(),
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
        Given::Diagnostics(text) => Ok(diagnostics_of(
            text,
            PythonSourceKind::Python,
            PythonParserSettings::default(),
        )),
        Given::ConfiguredDiagnostics(text, settings, source_kind) => {
            Ok(diagnostics_of(text, source_kind, settings))
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

fn diagnostics_of(
    text: &'static str,
    source_kind: PythonSourceKind,
    settings: PythonParserSettings,
) -> Seen {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder =
        languages::table_builder(&mut elements, &mut symbols, ByteSlice(text.as_bytes()));
    SyntaxRuffPythonParserAdapter.read(
        SourceText(text.as_bytes()),
        source_kind,
        &settings,
        &mut builder,
    );
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
    Seen::Diagnostics(Box::leak(diagnostics.into_boxed_slice()))
}

#[test]
fn succeeds() {
    check(&SyntaxRuffPythonParserAdapter);
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn parser_options_keep_source_kind_and_configured_target() {
    assert_eq!(
        super::adapter::source_type(PythonSourceKind::Python),
        PySourceType::Python
    );
    assert_eq!(
        super::adapter::source_type(PythonSourceKind::Stub),
        PySourceType::Stub
    );
    assert_eq!(
        super::adapter::options(
            PythonSourceKind::Python,
            &PythonParserSettings::new(PythonTargetVersion::Py310)
        )
        .target_version(),
        PythonVersion::PY310
    );
}
