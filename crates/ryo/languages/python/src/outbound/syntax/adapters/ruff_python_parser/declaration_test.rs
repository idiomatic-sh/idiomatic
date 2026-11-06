//! Compiler declaration validity is independent of stub typing or future recovery.

use super::adapter::SyntaxRuffPythonParserAdapter;
use super::declaration::semantic_syntax_errors;
use super::fixtures::diagnostics::{diagnostics, diagnostics_at, diagnostics_of};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::syntax::interface::SyntaxPort as _;
use crate::outbound::syntax::types::{PythonSourceKind, SourceText};
use crate::settings::{PythonParserSettings, PythonTargetVersion};
use ryo::languages::{self, ByteSlice, Elements, Symbols};

fn conflicting_declarations_are_rejected_in_either_order_and_in_stub_sources() {
    for source in [
        "def outer():\n value=0\n def inner():\n  global value\n  nonlocal value\n",
        "def outer():\n value=0\n def inner():\n  nonlocal value\n  global value\n",
        "def outer():\n value=0\n def inner():\n  if flag:\n   global value\n  nonlocal value\n",
        "def f(x:(value:=object)):\n global value\n nonlocal value\n",
    ] {
        let start = source.find("nonlocal value").unwrap() + "nonlocal ".len();
        for kind in [PythonSourceKind::Python, PythonSourceKind::Stub] {
            assert_eq!(
                diagnostics(source, kind),
                vec![(
                    "validation".to_owned(),
                    "preserved".to_owned(),
                    "nonlocal-global-conflict".to_owned(),
                    u32::try_from(start).unwrap(),
                    u32::try_from(start + "value".len()).unwrap()
                )],
                "{source}"
            );
        }
    }
}

fn separate_scopes_and_different_names_do_not_conflict() {
    for source in [
        "def outer():\n value=0\n def first():\n  global value\n def second():\n  nonlocal value\n",
        "def outer():\n value=0\n def inner():\n  global other\n  nonlocal value\n",
        "def f(x:(value:=object)): ...\n",
    ] {
        for kind in [PythonSourceKind::Python, PythonSourceKind::Stub] {
            assert!(diagnostics(source, kind).is_empty(), "{source}");
        }
    }
}

fn semantic_syntax_checks_only_run_after_an_accepted_parse() {
    let diagnostics_for = |source: &str| {
        let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
        let mut builder =
            languages::table_builder(&mut elements, &mut symbols, ByteSlice(source.as_bytes()));
        SyntaxRuffPythonParserAdapter::default().read(
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

fn comprehension_rebinding_preserves_ast_and_respects_lambda_scope() {
    let cases = [
        (
            "x=[data[index] for index,value in data.items() if (index:=0)]\n",
            Some((51, 56)),
        ),
        ("x=[(i:=0) for i in data]\n", Some((4, 5))),
        ("x=[[(i:=0) for j in other] for i in data]\n", Some((5, 6))),
        ("x=[(i:=0) for j in data for i in other]\n", Some((4, 5))),
        ("x=[lambda: (i:=0) for i in data]\n", None),
        ("x=[(obj:=0) for obj.attr in data]\n", None),
        ("x=[(other:=0) for i in data]\n", None),
        ("x={(i:=0) for i in data}\n", Some((4, 5))),
        ("x={i:(i:=0) for i in data}\n", Some((6, 7))),
        ("x=((i:=0) for i in data)\n", Some((4, 5))),
        (
            "x=[(lambda arg=(i:=0): arg) for i in data]\n",
            Some((16, 17)),
        ),
    ];
    for (source, expected) in cases {
        let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
        let mut builder =
            languages::table_builder(&mut elements, &mut symbols, ByteSlice(source.as_bytes()));
        SyntaxRuffPythonParserAdapter::default().read(
            SourceText(source.as_bytes()),
            PythonSourceKind::Python,
            &PythonParserSettings::new(PythonTargetVersion::Py310),
            &mut builder,
        );
        assert_eq!(
            languages::syntax_acceptance(&elements),
            languages::SyntaxAcceptance::Accepted,
            "{source}"
        );
        assert!(!languages::has_incomplete_syntax(&elements), "{source}");
        let diagnostics: Vec<_> = languages::diagnostics(&elements)
            .map(|diagnostic| {
                assert_eq!(diagnostic.phase.name(), "validation", "{source}");
                assert_eq!(diagnostic.impact.name(), "preserved", "{source}");
                assert_eq!(diagnostic.code.as_str(), "comprehension-rebind", "{source}");
                (diagnostic.start.0, diagnostic.end.0)
            })
            .collect();
        assert_eq!(
            diagnostics,
            expected.into_iter().collect::<Vec<_>>(),
            "{source}"
        );
    }
}

/// A module that declares no name, parameter or scope reports nothing.
fn a_module_without_declarations_reports_no_semantic_syntax_error() {
    let parsed = ruff_python_parser::parse_unchecked(
        "value = 1\n",
        ruff_python_parser::ParseOptions::from(ruff_python_ast::PySourceType::Python),
    );
    assert!(
        semantic_syntax_errors(
            parsed.syntax(),
            ruff_python_ast::PythonVersion::PY314,
            super::types::FutureAnnotations(false)
        )
        .is_empty()
    );
}

/// Which of the file's cases a row runs.
#[derive(Clone, Copy, Debug)]
enum Body {
    /// The case `a_module_without_declarations_reports_no_semantic_syntax_error`.
    AModuleWithoutDeclarationsReportsNoSemanticSyntaxError,
    /// The case `comprehension_rebinding_preserves_ast_and_respects_lambda_scope`.
    ComprehensionRebindingPreservesAstAndRespectsLambdaScope,
    /// The case `conflicting_declarations_are_rejected_in_either_order_and_in_stub_sources`.
    ConflictingDeclarationsAreRejectedInEitherOrderAndInStubSources,
    /// The case `separate_scopes_and_different_names_do_not_conflict`.
    SeparateScopesAndDifferentNamesDoNotConflict,
    /// The case `semantic_syntax_checks_only_run_after_an_accepted_parse`.
    SemanticSyntaxChecksOnlyRunAfterAnAcceptedParse,
}

/// The rows of the cases whose bodies assert for themselves.
const BODIES: &[Case<Body, (), ()>] = &[
    Case {
        name: CaseName("a_module_without_declarations_reports_no_semantic_syntax_error"),
        word: Word::Succeeds,
        input: || Body::AModuleWithoutDeclarationsReportsNoSemanticSyntaxError,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("comprehension_rebinding_preserves_ast_and_respects_lambda_scope"),
        word: Word::Succeeds,
        input: || Body::ComprehensionRebindingPreservesAstAndRespectsLambdaScope,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("conflicting_declarations_are_rejected_in_either_order_and_in_stub_sources"),
        word: Word::Succeeds,
        input: || Body::ConflictingDeclarationsAreRejectedInEitherOrderAndInStubSources,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("separate_scopes_and_different_names_do_not_conflict"),
        word: Word::Succeeds,
        input: || Body::SeparateScopesAndDifferentNamesDoNotConflict,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("semantic_syntax_checks_only_run_after_an_accepted_parse"),
        word: Word::Succeeds,
        input: || Body::SemanticSyntaxChecksOnlyRunAfterAnAcceptedParse,
        expect: Expect::Value(()),
    },
];

/// Runs the body of one case.
fn body(given: Body) -> Result<(), ()> {
    match given {
        Body::AModuleWithoutDeclarationsReportsNoSemanticSyntaxError => {
            a_module_without_declarations_reports_no_semantic_syntax_error()
        }
        Body::ComprehensionRebindingPreservesAstAndRespectsLambdaScope => {
            comprehension_rebinding_preserves_ast_and_respects_lambda_scope()
        }
        Body::ConflictingDeclarationsAreRejectedInEitherOrderAndInStubSources => {
            conflicting_declarations_are_rejected_in_either_order_and_in_stub_sources()
        }
        Body::SeparateScopesAndDifferentNamesDoNotConflict => {
            separate_scopes_and_different_names_do_not_conflict()
        }
        Body::SemanticSyntaxChecksOnlyRunAfterAnAcceptedParse => {
            semantic_syntax_checks_only_run_after_an_accepted_parse()
        }
    }
    Ok(())
}

/// A file's source text.
type Input = &'static str;
/// Each diagnostic's message and byte range, in report order.
type Value = &'static [(&'static str, u32, u32)];
type Failure = ();

/// The rows of the declaration checks on parameters, type parameters and keywords: what the adapter reports of a file, by message and
/// byte range.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("positional_only_keyword_only_and_variadic_parameters_can_be_distinct"),
        word: Word::Succeeds,
        input: || "def f(a, /, b, *args, c, **kwargs): ...\n",
        expect: Expect::Value(&[]),
    },
    Case {
        name: CaseName("duplicate_lambda_parameters_are_reported"),
        word: Word::Succeeds,
        input: || "lambda x, x: x\n",
        expect: Expect::Value(&[("Duplicate parameter \"x\"", 10, 11)]),
    },
    Case {
        name: CaseName("same_parameter_name_in_nested_functions_is_not_a_duplicate"),
        word: Word::Succeeds,
        input: || "def outer(x):\n    def inner(x): ...\n",
        expect: Expect::Value(&[]),
    },
    Case {
        name: CaseName("duplicate_type_parameters_report_each_later_occurrence"),
        word: Word::Succeeds,
        input: || "def f[T, T, T](): ...\n",
        expect: Expect::Value(&[
            ("duplicate type parameter", 9, 10),
            ("duplicate type parameter", 12, 13),
        ]),
    },
    Case {
        name: CaseName("non_default_type_parameters_after_a_default_are_reported"),
        word: Word::Succeeds,
        input: || "def f[T = int, U, V](): ...\n",
        expect: Expect::Value(&[
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
        ]),
    },
    Case {
        name: CaseName("a_fully_defaulted_type_parameter_suffix_is_valid"),
        word: Word::Succeeds,
        input: || "def f[T, U = int, V = str](): ...\n",
        expect: Expect::Value(&[]),
    },
    Case {
        name: CaseName("class_type_parameter_default_order_uses_its_local_scope"),
        word: Word::Succeeds,
        input: || "class C[T = int, U]: ...\n",
        expect: Expect::Value(&[(
            "non default type parameter `U` follows default type parameter",
            17,
            18,
        )]),
    },
    Case {
        name: CaseName("duplicate_call_keywords_report_the_second_keyword_range"),
        word: Word::Succeeds,
        input: || "call(x=1, x=2)\n",
        expect: Expect::Value(&[("Duplicate keyword argument `x`", 10, 13)]),
    },
    Case {
        name: CaseName("dictionary_expansion_keyword_duplicates_are_not_syntax_errors"),
        word: Word::Succeeds,
        input: || "call(x=1, **{\"x\": 2})\n",
        expect: Expect::Value(&[]),
    },
    Case {
        name: CaseName("duplicate_class_keywords_report_the_second_keyword_range"),
        word: Word::Succeeds,
        input: || "class C(metaclass=A, metaclass=B): ...\n",
        expect: Expect::Value(&[("Duplicate keyword argument `metaclass`", 21, 32)]),
    },
    Case {
        name: CaseName("duplicate_parameter_diagnostic_survives_an_unbound_name_in_the_body"),
        word: Word::Succeeds,
        input: || "def f(x, x):\n    print(missing)\n",
        expect: Expect::Value(&[("Duplicate parameter \"x\"", 9, 10)]),
    },
];

/// What the adapter reports of one source.
fn subject(text: Input) -> Result<Value, Failure> {
    Ok(diagnostics_of(
        text,
        PythonSourceKind::Python,
        PythonParserSettings::default(),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    run(BODIES, Word::Succeeds, body);
}
