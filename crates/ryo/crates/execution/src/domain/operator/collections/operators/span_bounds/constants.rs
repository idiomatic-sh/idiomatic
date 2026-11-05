//! Public description of checked physical span endpoints.

use crate::domain::describing::constants::{described, field};
use ryo_values::domain::descriptions::types::{FieldRequirement, OperatorDescription};

pub(crate) fn description() -> OperatorDescription {
    described(
        "span-bounds",
        "Append zero-based start and exclusive end byte offsets of each present ordered span. Input order and duplicates are preserved; absent, reversed, or out-of-file spans fail before their row emits.",
        vec![
            field("from", "Input rows.", FieldRequirement::Required),
            field(
                "span",
                "Physical span column to inspect.",
                FieldRequirement::Required,
            ),
            field(
                "start-output",
                "New zero-based start byte column.",
                FieldRequirement::Required,
            ),
            field(
                "end-output",
                "New exclusive end byte column.",
                FieldRequirement::Required,
            ),
        ],
        &[],
        r#"let example = span-bounds(from="function", span="span", start-output="begin", end-output="finish"); emit example subject file key(span);"#,
    )
}
