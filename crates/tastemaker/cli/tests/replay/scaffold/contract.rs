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
    for (method, relation) in [
        ("identifier", "Identifier"),
        ("qualified_name", "QualifiedName"),
        ("embedded_expression", "EmbeddedExpression"),
        ("expression_shape", "ExpressionShape"),
        ("literal_properties", "LiteralProperties"),
        ("format_specification", "FormatSpecification"),
        ("bytes_literal", "BytesLiteral"),
        ("interpolated_string_literal", "InterpolatedStringLiteral"),
        ("implicit_symbol", "ImplicitSymbol"),
        ("binding_event", "BindingEvent"),
        ("binding_read", "BindingRead"),
        ("binding_use", "BindingUse"),
        ("binding_shadow", "BindingShadow"),
        ("indent_region", "IndentRegion"),
    ] {
        let answer = format!(
            "fn {method}(&self) -> Support<Reader<languages::{relation}, Self::FileModel>> {{\n        languages::absent(AbsenceCause::NotYetRead)\n    }}"
        );
        assert!(
            contract.contains(&answer),
            "{method} must remain explicitly unread until implemented"
        );
    }
}
