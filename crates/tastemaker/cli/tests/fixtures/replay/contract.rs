//! Assertions for the generated language reader contract: no optional capability, the shared readers, and no optional
//! semantic relation answered.

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
    // The optional semantic relations (identifiers, bindings, indent regions and the rest) are answered through the
    // language's `Semantics`, and a fresh language implements none: it reads none of them until its author writes them.
    assert!(
        contract.contains(
            "fn semantics(&self) -> Option<&dyn languages::Semantics> {\n        None\n    }"
        ),
        "a fresh language must answer no optional semantic relation until implemented"
    );
    assert!(
        !contract.contains("impl languages::Semantics"),
        "a fresh language must implement no `Semantics` until its author writes one"
    );
}
