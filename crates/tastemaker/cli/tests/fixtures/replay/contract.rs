//! Assertions for the parse contract `tm scaffold language` writes: no optional capability, the shared readers, no optional
//! semantic relation answered and optional reader support explicitly unread.

use std::path::Path;

/// Keep optional reader support explicitly unread in a newly generated language.
#[expect(
    clippy::expect_used,
    reason = "test support: a generated language without its parse contract is a broken scaffold, and the panic names the file"
)]
pub(crate) fn assert_contract(language: &Path) {
    let contract =
        std::fs::read_to_string(language.join("src/inbound/contracts/parse/contract.rs"))
            .expect("the generated parse contract");
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
    assert!(
        !contract.contains("impl languages::Semantics"),
        "a fresh language must implement no `Semantics` until its author writes one"
    );
}
