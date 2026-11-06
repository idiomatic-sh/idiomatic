//! Compiler directives: `from __future__` imports are valid only in a module prefix, and only known features enable a mode.

use super::adapter::SyntaxRuffPythonParserAdapter;
use super::fixtures::diagnostics::{diagnostics, diagnostics_at};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::syntax::interface::SyntaxPort as _;
use crate::outbound::syntax::types::{PythonSourceKind, SourceText};
use crate::settings::{PythonParserSettings, PythonTargetVersion};
use ryo::languages::{self, ByteSlice, Elements, Symbols};

/// One independent compiler input and its expected module-mode and validity outcomes.
struct CompilerDirectiveCase {
    /// Complete source, identical for the Python and stub syntax acceptance checks.
    source: &'static str,
    /// Expected valid module-prefix future-annotations activation.
    enabled: bool,
    /// Exact source fragments whose byte spans receive validity diagnostics.
    refused_fragments: &'static [&'static str],
}

/// All original compiler controls, including relative, late, nested, lazy and generic negatives.
const COMPILER_DIRECTIVE_CASES: &[CompilerDirectiveCase] = &[
    CompilerDirectiveCase {
        source: "x:(value:=int)\n",
        enabled: false,
        refused_fragments: &[],
    },
    CompilerDirectiveCase {
        source: "from __future__ import annotations\nx:(value:=int)\n",
        enabled: true,
        refused_fragments: &["value:=int"],
    },
    CompilerDirectiveCase {
        source: "def f(x:(value:=int)): pass\n",
        enabled: false,
        refused_fragments: &[],
    },
    CompilerDirectiveCase {
        source: "from __future__ import annotations\ndef f(x:(value:=int)): pass\n",
        enabled: true,
        refused_fragments: &["value:=int"],
    },
    CompilerDirectiveCase {
        source: "\"module docstring\"\nfrom __future__ import annotations\ndef f(x:Missing): pass\n",
        enabled: true,
        refused_fragments: &[],
    },
    CompilerDirectiveCase {
        source: "from .__future__ import annotations\ndef f(x:(value:=int)): pass\n",
        enabled: false,
        refused_fragments: &[],
    },
    CompilerDirectiveCase {
        source: "x=1\nfrom __future__ import annotations\ndef f(x:(value:=int)): pass\n",
        enabled: false,
        refused_fragments: &["from __future__ import annotations"],
    },
    CompilerDirectiveCase {
        source: "def outer():\n from __future__ import annotations\n def f(x:(value:=int)): pass\n",
        enabled: false,
        refused_fragments: &["from __future__ import annotations"],
    },
    CompilerDirectiveCase {
        source: "from __future__ import annotations, not_a_feature\ndef f(x:Missing): pass\n",
        enabled: false,
        refused_fragments: &["not_a_feature"],
    },
    CompilerDirectiveCase {
        source: "lazy from __future__ import annotations\ndef f(x:(value:=int)): pass\n",
        enabled: false,
        refused_fragments: &["lazy", "lazy from __future__ import annotations"],
    },
    CompilerDirectiveCase {
        source: "def f[T](x:(value:=int)): pass\n",
        enabled: false,
        refused_fragments: &["value:=int"],
    },
    CompilerDirectiveCase {
        source: "class C[T]((value:=int)): pass\n",
        enabled: false,
        refused_fragments: &["value:=int"],
    },
    CompilerDirectiveCase {
        source: "def f[T:(value:=int)](): pass\n",
        enabled: false,
        refused_fragments: &["value:=int"],
    },
    CompilerDirectiveCase {
        source: "type Alias=(value:=int)\n",
        enabled: false,
        refused_fragments: &["value:=int"],
    },
    CompilerDirectiveCase {
        source: "type Alias=(lambda:(value:=int))\n",
        enabled: false,
        refused_fragments: &[],
    },
    CompilerDirectiveCase {
        source: "type Alias=(lambda x=(value:=int): x)\n",
        enabled: false,
        refused_fragments: &["value:=int"],
    },
    CompilerDirectiveCase {
        source: "from __future__ import annotations\ndef f(x:(lambda:(value:=int))): pass\n",
        enabled: true,
        refused_fragments: &[],
    },
    CompilerDirectiveCase {
        source: "from __future__ import annotations\ndef f(x:(lambda y=(value:=int):y)): pass\n",
        enabled: true,
        refused_fragments: &["value:=int"],
    },
];

/// Independent compiler controls; exact external observations are retained beside the fixture.

fn future_annotations_use_only_valid_compiler_directives() {
    for case in COMPILER_DIRECTIVE_CASES {
        let source = case.source;
        let refused_fragments = case.refused_fragments;
        for source_kind in [PythonSourceKind::Python, PythonSourceKind::Stub] {
            let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
            let mut builder =
                languages::table_builder(&mut elements, &mut symbols, ByteSlice(source.as_bytes()));
            SyntaxRuffPythonParserAdapter::default().read(
                SourceText(source.as_bytes()),
                source_kind,
                &PythonParserSettings::new(PythonTargetVersion::Py313),
                &mut builder,
            );
            assert_eq!(
                languages::property_at(
                    &elements,
                    languages::Position(0),
                    crate::domain::syntax::constants::MODULE_FUTURE_ANNOTATIONS
                )
                .map(|value| value.0),
                Some(if case.enabled { "enabled" } else { "disabled" }),
                "{source}"
            );
            let mut expected = refused_fragments
                .iter()
                .map(|fragment| {
                    let start = source
                        .find(*fragment)
                        .expect("the independently named fragment exists");
                    (
                        u32::try_from(start).expect("small fixture offset"),
                        u32::try_from(start + fragment.len()).expect("small fixture end"),
                    )
                })
                .collect::<Vec<_>>();
            expected.sort_unstable();
            let mut actual = languages::diagnostics(&elements)
                .map(|diagnostic| (diagnostic.start.0, diagnostic.end.0))
                .collect::<Vec<_>>();
            actual.sort_unstable();
            assert_eq!(actual, expected, "{source_kind:?}: {source}");
            assert!(!languages::has_incomplete_syntax(&elements), "{source}");
        }
    }
}

fn malformed_future_import_never_enables_a_compiler_feature() {
    let source = "from __future__ import (\n";
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder =
        languages::table_builder(&mut elements, &mut symbols, ByteSlice(source.as_bytes()));
    SyntaxRuffPythonParserAdapter::default().read(
        SourceText(source.as_bytes()),
        PythonSourceKind::Python,
        &PythonParserSettings::new(PythonTargetVersion::Py310),
        &mut builder,
    );
    assert!(languages::diagnostics(&elements).next().is_some());
    assert_ne!(
        languages::property_at(
            &elements,
            languages::Position(0),
            crate::domain::syntax::constants::MODULE_FUTURE_ANNOTATIONS
        )
        .map(|value| value.0),
        Some("enabled")
    );
}

fn future_lazy_prohibition_has_its_own_preserved_cause_at_old_and_supported_targets() {
    let statement = "lazy from __future__ import annotations";
    let source = format!("{statement}\ndef f(x:int): ...\n");
    for target in [PythonTargetVersion::Py310, PythonTargetVersion::Py315] {
        for kind in [PythonSourceKind::Python, PythonSourceKind::Stub] {
            let rows = diagnostics_at(&source, kind, target);
            let lazy: Vec<_> = rows
                .iter()
                .filter(|row| row.2 == "future-lazy-invalid")
                .collect();
            assert_eq!(lazy.len(), 1, "{target:?} {kind:?}: {rows:?}");
            assert_eq!(lazy[0].0, "validation");
            assert_eq!(lazy[0].1, "preserved");
            assert_eq!(
                (lazy[0].3, lazy[0].4),
                (0, u32::try_from(statement.len()).unwrap())
            );
            assert!(!rows.iter().any(|row| row.2 == "future-directive-invalid"));
            assert_eq!(
                rows.len(),
                if target == PythonTargetVersion::Py310 {
                    2
                } else {
                    1
                }
            );
        }
    }
}

fn separating_lazy_cause_preserves_placement_and_unknown_feature_causes() {
    for kind in [PythonSourceKind::Python, PythonSourceKind::Stub] {
        let placement = diagnostics_at(
            "x=1\nfrom __future__ import annotations\n",
            kind,
            PythonTargetVersion::Py315,
        );
        assert_eq!(placement.len(), 1);
        assert_eq!(placement[0].2, "future-directive-invalid");
        let unknown = diagnostics_at(
            "from __future__ import not_a_feature\n",
            kind,
            PythonTargetVersion::Py315,
        );
        assert_eq!(unknown.len(), 1);
        assert_eq!(unknown[0].2, "future-feature-not-defined");
        assert!(
            diagnostics_at(
                "lazy import collections\n",
                kind,
                PythonTargetVersion::Py315
            )
            .is_empty()
        );
    }
}

/// Which of the file's cases a row runs.
#[derive(Clone, Copy, Debug)]
enum Body {
    /// The case `separating_lazy_cause_preserves_placement_and_unknown_feature_causes`.
    SeparatingLazyCausePreservesPlacementAndUnknownFeatureCauses,
    /// The case `future_lazy_prohibition_has_its_own_preserved_cause_at_old_and_supported_targets`.
    FutureLazyProhibitionHasItsOwnPreservedCauseAtOldAndSupportedTargets,
    /// The case `future_annotations_use_only_valid_compiler_directives`.
    FutureAnnotationsUseOnlyValidCompilerDirectives,
    /// The case `malformed_future_import_never_enables_a_compiler_feature`.
    MalformedFutureImportNeverEnablesACompilerFeature,
}

/// The rows of the cases whose bodies assert for themselves.
const CASES: &[Case<Body, (), ()>] = &[
    Case {
        name: CaseName("separating_lazy_cause_preserves_placement_and_unknown_feature_causes"),
        word: Word::Succeeds,
        input: || Body::SeparatingLazyCausePreservesPlacementAndUnknownFeatureCauses,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "future_lazy_prohibition_has_its_own_preserved_cause_at_old_and_supported_targets",
        ),
        word: Word::Succeeds,
        input: || Body::FutureLazyProhibitionHasItsOwnPreservedCauseAtOldAndSupportedTargets,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("future_annotations_use_only_valid_compiler_directives"),
        word: Word::Succeeds,
        input: || Body::FutureAnnotationsUseOnlyValidCompilerDirectives,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("malformed_future_import_never_enables_a_compiler_feature"),
        word: Word::Succeeds,
        input: || Body::MalformedFutureImportNeverEnablesACompilerFeature,
        expect: Expect::Value(()),
    },
];

/// Runs the body of one case.
fn body(given: Body) -> Result<(), ()> {
    match given {
        Body::SeparatingLazyCausePreservesPlacementAndUnknownFeatureCauses => {
            separating_lazy_cause_preserves_placement_and_unknown_feature_causes()
        }
        Body::FutureLazyProhibitionHasItsOwnPreservedCauseAtOldAndSupportedTargets => {
            future_lazy_prohibition_has_its_own_preserved_cause_at_old_and_supported_targets()
        }
        Body::FutureAnnotationsUseOnlyValidCompilerDirectives => {
            future_annotations_use_only_valid_compiler_directives()
        }
        Body::MalformedFutureImportNeverEnablesACompilerFeature => {
            malformed_future_import_never_enables_a_compiler_feature()
        }
    }
    Ok(())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, body);
}
