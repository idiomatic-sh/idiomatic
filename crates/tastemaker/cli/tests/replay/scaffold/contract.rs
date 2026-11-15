//! Assertions for the generated language reader contract.

use std::path::Path;

/// Keep optional reader support explicitly unread in a newly generated language.
pub(super) fn assert_contract(language: &Path) {
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
    // `Semantics` since the generated Semantics trait: a fresh language implements none of them, so it answers `None`.
    assert!(
        contract.contains(
            "fn semantics(&self) -> Option<&dyn languages::Semantics> {\n        None\n    }"
        ),
        "a fresh language must not answer the optional semantic relations"
    );
    assert!(
        contract.contains("fn indent_region(&self) -> Support<Reader<languages::IndentRegion, Self::FileModel>> {\n        languages::absent(AbsenceCause::NotYetRead)\n    }"),
        "indent_region must remain explicitly unread until implemented"
    );
}
