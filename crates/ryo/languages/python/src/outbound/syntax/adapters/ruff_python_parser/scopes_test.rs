//! The scopes a file declares: each name a `global` or `nonlocal` binds, and the parameters and type parameters beside them, as the adapter reports them.

use super::fixtures::diagnostics::diagnostics_of;
use super::scopes::ScopeKind;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::syntax::types::PythonSourceKind;
use crate::settings::PythonParserSettings;

fn bare_starred_loop_target_preserves_accepted_ast_with_validation_error() {
    let source = "f=open(\"x\",\"w\")\nfor *[x] in rows:\n    f.write(x)\n";
    for target in [PythonTargetVersion::Py310, PythonTargetVersion::Py313] {
        for source_kind in [PythonSourceKind::Python, PythonSourceKind::Stub] {
            let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
            let mut builder =
                languages::table_builder(&mut elements, &mut symbols, ByteSlice(source.as_bytes()));
            SyntaxRuffPythonParserAdapter::default().read(
                SourceText(source.as_bytes()),
                source_kind,
                &PythonParserSettings::new(target),
                &mut builder,
            );
            assert_eq!(
                languages::syntax_acceptance(&elements),
                languages::SyntaxAcceptance::Accepted
            );
            assert!(!languages::has_incomplete_syntax(&elements));
            let diagnostics: Vec<_> = languages::diagnostics(&elements).collect();
            assert_eq!(diagnostics.len(), 1);
            let diagnostic = diagnostics.first().expect("one language validation error");
            assert_eq!(diagnostic.phase.name(), "validation");
            assert_eq!(diagnostic.impact.name(), "preserved");
            assert_eq!(diagnostic.code.as_str(), "starred-assignment-target");
            assert_eq!((diagnostic.start.0, diagnostic.end.0), (20, 24));
        }
    }
}

fn starred_targets_require_unpacking_containers_without_affecting_expression_stars() {
    let cases = [
        ("for *x in rows:\n pass\n", false),
        ("for (*[x]) in rows:\n pass\n", false),
        ("π=0\nfor *[x] in rows:\n pass\n", false),
        ("*[x] = rows\n", false),
        ("with resource() as *[x]:\n pass\n", false),
        ("values=[x for *[x] in rows]\n", false),
        ("async def f():\n async for *[x] in rows:\n  pass\n", false),
        ("for (*[x],) in rows:\n pass\n", true),
        ("for [*[x]] in rows:\n pass\n", true),
        ("for *x, in rows:\n pass\n", true),
        ("(*[x],) = rows\n", true),
        ("with resource() as (*[x],):\n pass\n", true),
        ("values=[x for (*[x],) in rows]\n", true),
        ("values=[*rows]\nfn(*rows)\nvalue=a*b\n", true),
    ];
    for (source, valid) in cases {
        for target in [PythonTargetVersion::Py310, PythonTargetVersion::Py313] {
            let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
            let mut builder =
                languages::table_builder(&mut elements, &mut symbols, ByteSlice(source.as_bytes()));
            SyntaxRuffPythonParserAdapter::default().read(
                SourceText(source.as_bytes()),
                PythonSourceKind::Python,
                &PythonParserSettings::new(target),
                &mut builder,
            );
            let diagnostics: Vec<_> = languages::diagnostics(&elements).collect();
            if valid {
                assert!(diagnostics.is_empty(), "valid unpacking context: {source}");
                assert_eq!(
                    languages::syntax_acceptance(&elements),
                    languages::SyntaxAcceptance::Accepted
                );
            } else {
                assert!(
                    !diagnostics.is_empty(),
                    "bare target must be refused: {source}"
                );
                if languages::syntax_acceptance(&elements) == languages::SyntaxAcceptance::Accepted
                {
                    assert!(
                        diagnostics
                            .iter()
                            .any(|row| row.code.as_str() == "starred-assignment-target")
                    );
                    assert!(!languages::has_incomplete_syntax(&elements));
                } else {
                    assert!(
                        !diagnostics
                            .iter()
                            .any(|row| row.code.as_str() == "starred-assignment-target"),
                        "the parser already refused this input; do not duplicate validation"
                    );
                }
            }
        }
    }
}

/// Which of the file's cases a row runs.
#[derive(Clone, Copy, Debug)]
enum Body {
    /// The case `bare_starred_loop_target_preserves_accepted_ast_with_validation_error`.
    BareStarredLoopTargetPreservesAcceptedAstWithValidationError,
    /// The case `starred_targets_require_unpacking_containers_without_affecting_expression_stars`.
    StarredTargetsRequireUnpackingContainersWithoutAffectingExpressionStars,
}

/// The rows of the cases whose bodies assert for themselves.
const BODIES: &[Case<Body, (), ()>] = &[
    Case {
        name: CaseName("bare_starred_loop_target_preserves_accepted_ast_with_validation_error"),
        word: Word::Succeeds,
        input: || Body::BareStarredLoopTargetPreservesAcceptedAstWithValidationError,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "starred_targets_require_unpacking_containers_without_affecting_expression_stars",
        ),
        word: Word::Succeeds,
        input: || Body::StarredTargetsRequireUnpackingContainersWithoutAffectingExpressionStars,
        expect: Expect::Value(()),
    },
];

/// Runs the body of one case.
fn body(given: Body) -> Result<(), ()> {
    match given {
        Body::ComprehensionRebindingPreservesAstAndRespectsLambdaScope => {
            comprehension_rebinding_preserves_ast_and_respects_lambda_scope()
        }
        Body::BareStarredLoopTargetPreservesAcceptedAstWithValidationError => {
            bare_starred_loop_target_preserves_accepted_ast_with_validation_error()
        }
        Body::StarredTargetsRequireUnpackingContainersWithoutAffectingExpressionStars => {
            starred_targets_require_unpacking_containers_without_affecting_expression_stars()
        }
    }
    Ok(())
}

/// A file's source text.
type Input = &'static str;
/// Each diagnostic's message and byte range, in report order.
type Value = &'static [(&'static str, u32, u32)];
type Failure = ();

/// The rows of the declaration checks: what the adapter reports of a file, by message and byte range.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_module_nonlocal_has_the_pinned_semantic_syntax_error"),
        word: Word::Succeeds,
        input: || "nonlocal x\n",
        expect: Expect::Value(&[("nonlocal declaration not allowed at module level", 0, 10)]),
    },
    Case {
        name: CaseName("a_nonlocal_after_global_has_the_pinned_statement_range"),
        word: Word::Succeeds,
        input: || "global x\nnonlocal x\n",
        expect: Expect::Value(&[("nonlocal declaration not allowed at module level", 9, 19)]),
    },
    Case {
        name: CaseName("a_nonlocal_in_a_module_compound_statement_is_still_module_scoped"),
        word: Word::Succeeds,
        input: || "if True:\n    nonlocal x\n",
        expect: Expect::Value(&[("nonlocal declaration not allowed at module level", 13, 23)]),
    },
    Case {
        name: CaseName("a_nonlocal_in_a_module_loop_is_still_module_scoped"),
        word: Word::Succeeds,
        input: || "for x in xs:\n    nonlocal x\n",
        expect: Expect::Value(&[("nonlocal declaration not allowed at module level", 17, 27)]),
    },
    Case {
        name: CaseName("a_nested_nonlocal_does_not_get_the_module_scope_diagnostic"),
        word: Word::Succeeds,
        input: || "def f():\n    nonlocal x\n",
        expect: Expect::Value(&[]),
    },
    Case {
        name: CaseName("a_top_level_class_nonlocal_gets_no_module_scope_diagnostic"),
        word: Word::Succeeds,
        input: || "class C:\n    nonlocal x\n",
        expect: Expect::Value(&[]),
    },
    Case {
        name: CaseName("a_class_inside_a_function_can_use_an_enclosing_nonlocal"),
        word: Word::Succeeds,
        input: || "def outer():\n    x = 1\n    class C:\n        nonlocal x\n",
        expect: Expect::Value(&[]),
    },
    Case {
        name: CaseName("duplicate_function_parameters_report_each_repeated_name"),
        word: Word::Succeeds,
        input: || "def f(a, /, a, *, b, b): ...\n",
        expect: Expect::Value(&[
            ("Duplicate parameter \"a\"", 12, 13),
            ("Duplicate parameter \"b\"", 21, 22),
        ]),
    },
    Case {
        name: CaseName("global_parameter_errors_follow_the_declaration_name"),
        word: Word::Succeeds,
        input: || "def f(x):\n    global x\n",
        expect: Expect::Value(&[(
            "name `x` cannot refer to a parameter and a global variable",
            21,
            22,
        )]),
    },
    Case {
        name: CaseName("nonlocal_parameter_errors_even_with_an_outer_binding"),
        word: Word::Succeeds,
        input: || "def outer():\n    x = 1\n    def inner(x):\n        nonlocal x\n",
        expect: Expect::Value(&[(
            "name `x` cannot refer to a parameter and a nonlocal variable",
            58,
            59,
        )]),
    },
    Case {
        name: CaseName("each_repeated_global_parameter_declaration_is_reported"),
        word: Word::Succeeds,
        input: || "def f(x):\n    global x\n    global x\n",
        expect: Expect::Value(&[
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
        ]),
    },
    Case {
        name: CaseName("global_and_annotation_diagnostics_follow_source_order"),
        word: Word::Succeeds,
        input: || "def f(x):\n    x: int\n    global x\n    x: str\n",
        expect: Expect::Value(&[
            ("annotated name `x` can't be global", 14, 15),
            (
                "name `x` cannot refer to a parameter and a global variable",
                32,
                33,
            ),
            ("annotated name `x` can't be global", 38, 39),
        ]),
    },
    Case {
        name: CaseName("a_module_annotation_before_repeated_globals_is_reported_once"),
        word: Word::Succeeds,
        input: || "x: int\nglobal x\nglobal x\n",
        expect: Expect::Value(&[("annotated name `x` can't be global", 0, 1)]),
    },
    Case {
        name: CaseName("a_module_annotation_after_global_is_allowed"),
        word: Word::Succeeds,
        input: || "global x\nx: int\n",
        expect: Expect::Value(&[]),
    },
    Case {
        name: CaseName("a_later_module_global_makes_an_intervening_annotation_invalid"),
        word: Word::Succeeds,
        input: || "global x\nx: int\nglobal x\n",
        expect: Expect::Value(&[("annotated name `x` can't be global", 9, 10)]),
    },
    Case {
        name: CaseName("nested_scope_declarations_do_not_leak_across_function_class_or_lambda"),
        word: Word::Succeeds,
        input: || "def outer(x):\n    global x\n    class C:\n        y: int\n        def inner(y):\n            global y\n    callback = lambda x: x\n",
        expect: Expect::Value(&[
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
        ]),
    },
    Case {
        name: CaseName("a_class_scope_global_annotation_is_not_the_enclosing_function_global"),
        word: Word::Succeeds,
        input: || "def f(x):\n    class C:\n        global x\n        x: int\n",
        expect: Expect::Value(&[("annotated name `x` can't be global", 48, 49)]),
    },
    Case {
        name: CaseName("unicode_global_parameter_ranges_are_utf8_bytes"),
        word: Word::Succeeds,
        input: || "def f(π):\n    global π\n",
        expect: Expect::Value(&[(
            "name `π` cannot refer to a parameter and a global variable",
            22,
            24,
        )]),
    },
    Case {
        name: CaseName("unicode_annotated_global_ranges_are_utf8_bytes"),
        word: Word::Succeeds,
        input: || "def f():\n    global π\n    π: int\n",
        expect: Expect::Value(&[("annotated name `π` can't be global", 27, 29)]),
    },
    Case {
        name: CaseName("module_nonlocal_is_reported_without_other_module_scope_errors"),
        word: Word::Succeeds,
        input: || "global x\nnonlocal y\nx: int\n",
        expect: Expect::Value(&[("nonlocal declaration not allowed at module level", 9, 19)]),
    },
    Case {
        name: CaseName("global_indexes_are_local_to_the_function_scope"),
        word: Word::Succeeds,
        input: || "global x\ndef f():\n    x: int\n",
        expect: Expect::Value(&[]),
    },
    Case {
        name: CaseName("a_nested_function_global_does_not_affect_its_parent_annotation"),
        word: Word::Succeeds,
        input: || "def outer():\n    def inner():\n        global x\n    x: int\n",
        expect: Expect::Value(&[]),
    },
    Case {
        name: CaseName("each_repeated_nonlocal_parameter_declaration_is_reported"),
        word: Word::Succeeds,
        input: || "def outer():\n    x = 1\n    def inner(x):\n        nonlocal x\n        nonlocal x\n",
        expect: Expect::Value(&[
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
        ]),
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

/// The three kinds of scope are distinct.
const KINDS: [ScopeKind; 3] = [ScopeKind::Module, ScopeKind::Function, ScopeKind::Class];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    run(BODIES, Word::Succeeds, body);
    assert!(KINDS[0] != KINDS[1] && KINDS[1] != KINDS[2] && KINDS[0] != KINDS[2]);
}
