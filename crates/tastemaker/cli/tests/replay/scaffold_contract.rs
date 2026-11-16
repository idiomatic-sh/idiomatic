//! What the `scaffold` row holds the parse contract `tm scaffold language` writes to: optional reader support explicitly unread.

/// Keep optional reader support explicitly unread in a newly generated language, given the text of its parse contract.
pub(crate) fn assert_contract(contract: &str) {
    assert!(
        contract.contains("fn capabilities(&self) -> &'static [languages::OptionalCapability] {\n        &[]\n    }"),
        "a fresh language must not advertise unimplemented optional capabilities"
    );
    for reader in [
        "native_semantic_syntax",
        "native_semantic_child",
        "source_locations",
    ] {
        assert!(
            contract.contains(&format!("languages::{reader}()")),
            "missing shared reader {reader}"
        );
    }
    // The optional semantic relations (identifiers, qualified names, bindings, expression shapes, ...) are answered through
    // `Semantics` since the generated Semantics trait: a fresh language implements none of them, so it answers `Absent`.
    assert!(
        contract.contains(
            "fn semantics(&self) -> languages::SemanticAnswers<'_> {\n        languages::SemanticAnswers::Absent\n    }"
        ),
        "a fresh language must not answer the optional semantic relations"
    );
    assert!(
        contract.contains("fn indent_region(&self) -> Support<Reader<languages::IndentRegion, Self::FileModel>> {\n        languages::absent(AbsenceCause::NotYetRead)\n    }"),
        "indent_region must remain explicitly unread until implemented"
    );
}
