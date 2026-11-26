//! Compiler directives: `from __future__` imports are valid only in a module prefix, and only known features enable a mode.

use super::adapter::SyntaxRuffPythonParserAdapter;
use super::fixtures::diagnostics::{diagnostics_at};
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
            SyntaxRuffPythonParserAdapter {
                settings: PythonParserSettings::new(PythonTargetVersion::Py313),
                ..Default::default()
            }
            .read(SourceText(source.as_bytes()), source_kind, &mut builder);
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
            assert!(
                languages::has_incomplete_syntax(&elements)
                    == languages::SyntaxCompleteness::Complete,
                "{source}"
            );
        }
    }
}

fn malformed_future_import_never_enables_a_compiler_feature() {
    let source = "from __future__ import (\n";
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder =
        languages::table_builder(&mut elements, &mut symbols, ByteSlice(source.as_bytes()));
    SyntaxRuffPythonParserAdapter {
        settings: PythonParserSettings::new(PythonTargetVersion::Py310),
        ..Default::default()
    }
    .read(
        SourceText(source.as_bytes()),
        PythonSourceKind::Python,
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

/// The `binding-use` rows of `source` read under `path` at Python 3.10.
fn uses_of(path: &str, source: &'static str) -> usize {
    let report = languages::contract::reading::<
        crate::domain::relations::collections::relations::binding_use::item::BindingUseItem,
    >(
        &crate::linked_with_settings(PythonParserSettings::new(PythonTargetVersion::Py310)),
        languages::SourcePath(path),
        languages::SampleText(source),
    )
    .expect("Python source is recognized");
    assert!(report.causes.is_empty(), "{:?}", report.causes);
    report.rows.len()
}

/// The `binding-use` rows the walk writes of `source` under `path`.
fn walked_uses_of(path: &str, source: &'static str) -> usize {
    use crate::fixtures::walking::{walk_words, walked, walked_keys, walked_rows};
    let (mut table, mut scratch, mut count) = (languages::NodeTable::default(), Default::default(), None);
    walked(
        &crate::walk(),
        path,
        source.as_bytes(),
        &[languages::Relation::BindingUse],
        &mut table,
        &mut scratch,
        &mut |mut file| {
            let mut words = walk_words(&crate::walk()).expect("each fixed name of the walk is stated once");
            let keys = walked_keys(file.table);
            let (rows, answer) = walked_rows(&mut file, &keys, languages::Relation::BindingUse, &mut words);
            assert!(matches!(answer, languages::FactAnswer::Answered(_)), "{answer:?}");
            count = Some(rows.len());
        },
    )
    .expect("Python source is walked");
    count.expect("the walk lends the file")
}

/// A stub is never run, and the typing specification reads its `x: T` as the declaration of `x`: a read after an annotated
/// declaration without a value uses it, at module, function and class level (three uses), by today's reading and the walk
/// alike; in a module that runs the declaration gives the name no value, and the reads use nothing.
fn stub_declarations_bind_the_names_they_declare() {
    const DECLARED: &str = "codec: str\ncodec.encode\ndef f() -> None:\n    local: int\n    local.bit_length\nclass C:\n    attr: int\n    x = attr\n";
    let counts = (
        uses_of("case.py", DECLARED),
        uses_of("case.pyi", DECLARED),
        walked_uses_of("case.py", DECLARED),
        walked_uses_of("case.pyi", DECLARED),
    );
    assert_eq!(counts, (0, 3, 0, 3));
}

/// A compile-time function local masks its outer homonym before a store or deletion.
#[test]
fn uninitialized_locals_have_no_outer_use() {
    for source in [
        "x = 1\ndef f():\n    print(x)\n    x = 2\n",
        "x = 1\ndef f():\n    print(x)\n    del x\n",
        "x = 1\ndef f():\n    print(x)\n    x: int\n",
    ] {
        assert_eq!(uses_of("case.py", source), 0);
        assert_eq!(walked_uses_of("case.py", source), 0);
    }
}

/// Which of the file's cases a row runs.
#[derive(Clone, Copy, Debug)]
enum Body {
    /// Stub declarations are readable through both provider entry points.
    StubDeclarationsBindTheNamesTheyDeclare,
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
        name: CaseName("stub_declarations_bind_the_names_they_declare"),
        word: Word::Succeeds,
        input: || Body::StubDeclarationsBindTheNamesTheyDeclare,
        expect: Expect::Value(()),
    },
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
        Body::StubDeclarationsBindTheNamesTheyDeclare => stub_declarations_bind_the_names_they_declare(),
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
