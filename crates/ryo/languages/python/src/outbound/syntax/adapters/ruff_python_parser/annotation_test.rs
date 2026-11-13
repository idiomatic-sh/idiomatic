//! Named expressions are restricted in deferred and generic annotation scopes.

use super::adapter::SyntaxRuffPythonParserAdapter;
use super::annotation::annotation_named_expression_syntax_errors;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::syntax::interface::SyntaxPort as _;
use crate::outbound::syntax::types::{PythonSourceKind, SourceText};
use crate::settings::{PythonParserSettings, PythonTargetVersion};
use ruff_python_ast::PythonVersion;
use ryo::languages::{self, ByteSlice, Elements, Symbols};

fn named_expression_annotation_validity_does_not_depend_on_filename() {
    let diagnostics_for = |source: &str, source_kind| {
        let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
        let mut builder =
            languages::table_builder(&mut elements, &mut symbols, ByteSlice(source.as_bytes()));
        SyntaxRuffPythonParserAdapter {
            settings: PythonParserSettings::new(PythonTargetVersion::Py310),
            ..Default::default()
        }
        .read(SourceText(source.as_bytes()), source_kind, &mut builder);
        let diagnostics: Vec<_> = languages::diagnostics(&elements)
            .map(|diagnostic| {
                (
                    languages::phase_name(diagnostic.phase).0.to_owned(),
                    languages::impact_name(diagnostic.impact).0.to_owned(),
                    diagnostic.code.0.to_owned(),
                    diagnostic.start.0,
                    diagnostic.end.0,
                )
            })
            .collect();
        (
            languages::syntax_acceptance(&elements),
            languages::has_incomplete_syntax(&elements)
                == languages::SyntaxCompleteness::Incomplete,
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
    assert!(stub_diagnostics.is_empty());

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

fn native_annotation_scopes_are_version_sensitive() {
    let source = "x:(value:=int)\n";
    for (target, reject) in [
        (PythonTargetVersion::Py313, false),
        (PythonTargetVersion::Py314, true),
        (PythonTargetVersion::Py315, true),
    ] {
        let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
        let mut builder =
            languages::table_builder(&mut elements, &mut symbols, ByteSlice(source.as_bytes()));
        SyntaxRuffPythonParserAdapter {
            settings: PythonParserSettings::new(target),
            ..Default::default()
        }
        .read(
            SourceText(source.as_bytes()),
            PythonSourceKind::Python,
            &mut builder,
        );
        let actual = languages::diagnostics(&elements)
            .map(|diagnostic| (diagnostic.start.0, diagnostic.end.0))
            .collect::<Vec<_>>();
        assert_eq!(actual, if reject { vec![(3, 13)] } else { Vec::new() });
        assert_eq!(
            languages::property_at(
                &elements,
                languages::Position(0),
                crate::domain::syntax::constants::MODULE_FUTURE_ANNOTATIONS
            )
            .map(|value| value.0),
            Some("disabled")
        );
    }
}

/// A module with no annotation reports nothing, at any target.
fn a_module_without_annotations_reports_no_named_expression_error() {
    let parsed =
        ruff_python_parser::parse_module("value = 1\n").expect("a plain assignment parses");
    assert!(
        annotation_named_expression_syntax_errors(
            parsed.syntax(),
            PythonVersion::PY314,
            super::types::FutureAnnotations(false),
        )
        .is_empty()
    );
}

/// Which of the file's cases a row runs.
#[derive(Clone, Copy, Debug)]
enum Body {
    /// The case `named_expression_annotation_validity_does_not_depend_on_filename`.
    NamedExpressionAnnotationValidityDoesNotDependOnFilename,
    /// The case `a_module_without_annotations_reports_no_named_expression_error`.
    AModuleWithoutAnnotationsReportsNoNamedExpressionError,
    /// The case `native_annotation_scopes_are_version_sensitive`.
    NativeAnnotationScopesAreVersionSensitive,
}

/// The rows of the cases whose bodies assert for themselves.
const CASES: &[Case<Body, (), ()>] = &[
    Case {
        name: CaseName("a_module_without_annotations_reports_no_named_expression_error"),
        word: Word::Succeeds,
        input: || Body::AModuleWithoutAnnotationsReportsNoNamedExpressionError,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("named_expression_annotation_validity_does_not_depend_on_filename"),
        word: Word::Succeeds,
        input: || Body::NamedExpressionAnnotationValidityDoesNotDependOnFilename,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("native_annotation_scopes_are_version_sensitive"),
        word: Word::Succeeds,
        input: || Body::NativeAnnotationScopesAreVersionSensitive,
        expect: Expect::Value(()),
    },
];

/// Runs the body of one case.
fn body(given: Body) -> Result<(), ()> {
    match given {
        Body::AModuleWithoutAnnotationsReportsNoNamedExpressionError => {
            a_module_without_annotations_reports_no_named_expression_error();
        }
        Body::NamedExpressionAnnotationValidityDoesNotDependOnFilename => {
            named_expression_annotation_validity_does_not_depend_on_filename()
        }
        Body::NativeAnnotationScopesAreVersionSensitive => {
            native_annotation_scopes_are_version_sensitive()
        }
    }
    Ok(())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, body);
}
