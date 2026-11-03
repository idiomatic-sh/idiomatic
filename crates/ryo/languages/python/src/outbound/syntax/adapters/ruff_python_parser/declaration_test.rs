//! Compiler declaration validity is independent of stub typing or future recovery.
use super::adapter::SyntaxRuffPythonParserAdapter;
use crate::outbound::syntax::{
    interface::SyntaxPort as _,
    types::{PythonSourceKind, SourceText},
};
use crate::settings::{PythonParserSettings, PythonTargetVersion};
use ryo::languages::{self, ByteSlice, Elements, Symbols};

fn diagnostics(source: &str, kind: PythonSourceKind) -> Vec<(String, String, String, u32, u32)> {
    diagnostics_at(source, kind, PythonTargetVersion::Py310)
}

fn diagnostics_at(
    source: &str,
    kind: PythonSourceKind,
    target: PythonTargetVersion,
) -> Vec<(String, String, String, u32, u32)> {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder =
        languages::table_builder(&mut elements, &mut symbols, ByteSlice(source.as_bytes()));
    SyntaxRuffPythonParserAdapter.read(
        SourceText(source.as_bytes()),
        kind,
        &PythonParserSettings::new(target),
        &mut builder,
    );
    assert_eq!(
        languages::syntax_acceptance(&elements),
        languages::SyntaxAcceptance::Accepted
    );
    assert!(
        !languages::has_incomplete_syntax(&elements),
        "declaration error must preserve complete AST facts"
    );
    languages::diagnostics(&elements)
        .map(|diagnostic| {
            (
                diagnostic.phase.name().to_owned(),
                diagnostic.impact.name().to_owned(),
                diagnostic.code.as_str().to_owned(),
                diagnostic.start.0,
                diagnostic.end.0,
            )
        })
        .collect()
}

#[test]
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

#[test]
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

#[test]
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

#[test]
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
