//! The parser options a run's settings and a file's kind select.

use super::adapter::SyntaxRuffPythonParserAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::syntax::interface::SyntaxPort as _;
use crate::outbound::syntax::types::{PythonSourceKind, SourceText};
use crate::settings::{PythonParserSettings, PythonTargetVersion};
use ruff_python_ast::{PySourceType, PythonVersion};
use ryo::languages::{self, ByteSlice, Elements, Symbols};

fn parser_options_keep_source_kind_and_configured_target() {
    assert_eq!(
        super::options::source_type(PythonSourceKind::Python),
        PySourceType::Python
    );
    assert_eq!(
        super::options::source_type(PythonSourceKind::Stub),
        PySourceType::Stub
    );
    assert_eq!(
        super::options::options(
            PythonSourceKind::Python,
            &PythonParserSettings::new(PythonTargetVersion::Py310)
        )
        .target_version(),
        PythonVersion::PY310
    );
}

fn target_version_syntax_warning_preserves_the_tree() {
    let source = "try:\n    pass\nexcept* (ValueError,):\n    pass\n";
    let diagnostics_for = |target| {
        let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
        let mut builder =
            languages::table_builder(&mut elements, &mut symbols, ByteSlice(source.as_bytes()));
        SyntaxRuffPythonParserAdapter::default().read(
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

/// Which of the file's cases a row runs.
#[derive(Clone, Copy, Debug)]
enum Body {
    /// The case `target_version_syntax_warning_preserves_the_tree`.
    TargetVersionSyntaxWarningPreservesTheTree,
    /// The case `parser_options_keep_source_kind_and_configured_target`.
    ParserOptionsKeepSourceKindAndConfiguredTarget,
}

/// The rows of the cases whose bodies assert for themselves.
const CASES: &[Case<Body, (), ()>] = &[
    Case {
        name: CaseName("target_version_syntax_warning_preserves_the_tree"),
        word: Word::Succeeds,
        input: || Body::TargetVersionSyntaxWarningPreservesTheTree,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("parser_options_keep_source_kind_and_configured_target"),
        word: Word::Succeeds,
        input: || Body::ParserOptionsKeepSourceKindAndConfiguredTarget,
        expect: Expect::Value(()),
    },
];

/// Runs the body of one case.
fn body(given: Body) -> Result<(), ()> {
    match given {
        Body::TargetVersionSyntaxWarningPreservesTheTree => {
            target_version_syntax_warning_preserves_the_tree()
        }
        Body::ParserOptionsKeepSourceKindAndConfiguredTarget => {
            parser_options_keep_source_kind_and_configured_target()
        }
    }
    Ok(())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, body);
}
