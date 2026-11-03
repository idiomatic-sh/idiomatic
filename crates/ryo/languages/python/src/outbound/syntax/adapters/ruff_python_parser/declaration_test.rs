//! Compiler declaration validity is independent of stub typing or future recovery.
use super::adapter::SyntaxRuffPythonParserAdapter;
use crate::outbound::syntax::{
    interface::SyntaxPort as _,
    types::{PythonSourceKind, SourceText},
};
use crate::settings::{PythonParserSettings, PythonTargetVersion};
use ryo::languages::{self, ByteSlice, Elements, Symbols};

fn diagnostics(source: &str, kind: PythonSourceKind) -> Vec<(String, String, String, u32, u32)> {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder =
        languages::table_builder(&mut elements, &mut symbols, ByteSlice(source.as_bytes()));
    SyntaxRuffPythonParserAdapter.read(
        SourceText(source.as_bytes()),
        kind,
        &PythonParserSettings::new(PythonTargetVersion::Py310),
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
