//! Public description of the byte-span projection.

use crate::domain::describing::constants::{described, field};
use ryo_values::domain::descriptions::types::{FieldRequirement, OperatorDescription};

/// Inputs and example for a per-file span composition.
pub(crate) fn description() -> OperatorDescription {
    described(
        "span-between",
        "Append [start.start, end.end) in the current file, preserving every input row. Absent spans propagate; invalid byte ranges fail.",
        vec![
            field("from", "Per-file input rows.", FieldRequirement::Required),
            field(
                "start",
                "Span column supplying the start offset.",
                FieldRequirement::Required,
            ),
            field(
                "end",
                "Span column supplying the end offset.",
                FieldRequirement::Required,
            ),
            field("output", "New span column.", FieldRequirement::Required),
        ],
        &[],
        r#"let example = span-between(from="function", start="span", end="span", output="diagnostic_span"); emit example subject file key(diagnostic_span);"#,
    )
}
