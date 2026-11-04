//! Canonical authored declarations shared by invocation identity and durable case receipts.
use super::types::RuleObservationDeclarations;

/// JSON records the authored data, with no inferred producers or exported diagnostics.
pub(crate) fn value(declarations: &RuleObservationDeclarations) -> serde_json::Value {
    use ryo::linter::{ObservationKind, ObservationProducer};
    let entries = declarations
        .iter()
        .map(|(rule, bindings)| {
            let bindings: Vec<_> = bindings.iter().map(|binding| {
            let columns: Vec<_> = binding.columns.iter().map(|column| serde_json::json!({
                "name": column.name.0,
                "kind": match column.kind {
                    ObservationKind::Node => "node",
                    ObservationKind::Symbol => "symbol",
                    ObservationKind::File => "file",
                    ObservationKind::Span => "span",
                    ObservationKind::Number => "number",
                    ObservationKind::Flag => "flag",
                },
            })).collect();
            let producers: Vec<_> = binding.producers.iter().map(|producer| match producer {
                ObservationProducer::Output { rule, output } => serde_json::json!({
                    "kind": "output", "rule": rule.as_ref(), "output": output.0,
                }),
                ObservationProducer::NoQuery { rule } => serde_json::json!({
                    "kind": "no-query", "rule": rule.as_ref(),
                }),
            }).collect();
            serde_json::json!({"name": binding.name.0, "columns": columns, "producers": producers})
        }).collect();
            (rule.to_string(), serde_json::Value::Array(bindings))
        })
        .collect();
    serde_json::Value::Object(entries)
}
