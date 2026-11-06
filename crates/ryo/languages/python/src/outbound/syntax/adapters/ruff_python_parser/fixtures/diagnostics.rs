//! What the adapter reports of a file, as the tests read it: each diagnostic's message and byte range, in report order.

use super::super::adapter::SyntaxRuffPythonParserAdapter;
use crate::outbound::syntax::interface::SyntaxPort as _;
use crate::outbound::syntax::types::{PythonSourceKind, SourceText};
use crate::settings::{PythonParserSettings, PythonTargetVersion};
use ryo::languages::{self, ByteSlice, Elements, Symbols};

/// What the adapter reports of `text` read as `source_kind` under `settings`: each message with its byte range.
pub(crate) fn diagnostics_of(
    text: &'static str,
    source_kind: PythonSourceKind,
    settings: PythonParserSettings,
) -> &'static [(&'static str, u32, u32)] {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder =
        languages::table_builder(&mut elements, &mut symbols, ByteSlice(text.as_bytes()));
    SyntaxRuffPythonParserAdapter::default().read(
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
    Box::leak(diagnostics.into_boxed_slice())
}

/// The phase, impact, code and byte range of each diagnostic the adapter reports for `source` at the default target.
pub(crate) fn diagnostics(
    source: &str,
    kind: PythonSourceKind,
) -> Vec<(String, String, String, u32, u32)> {
    diagnostics_at(source, kind, PythonTargetVersion::Py310)
}

/// The phase, impact, code and byte range of each diagnostic the adapter reports for `source` at `target`; the tree must be accepted and complete.
pub(crate) fn diagnostics_at(
    source: &str,
    kind: PythonSourceKind,
    target: PythonTargetVersion,
) -> Vec<(String, String, String, u32, u32)> {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder =
        languages::table_builder(&mut elements, &mut symbols, ByteSlice(source.as_bytes()));
    SyntaxRuffPythonParserAdapter::default().read(
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
