//! Public description and example of this operator.

use crate::domain::describing::constants::{described, field};
use ryo_values::domain::descriptions::types::{FieldRequirement, OperatorDescription};

/// Describe this operator's inputs, output and independent example.
pub(crate) fn description() -> OperatorDescription {
    described(
        "replace",
        "Append text with literal replacement of regex matches.",
        vec![
            field("from", "Input rows.", FieldRequirement::Required),
            field("text", "Text column.", FieldRequirement::Required),
            field(
                ryo_values::domain::descriptions::constants::PATTERN,
                "Constant regular expression.",
                FieldRequirement::Required,
            ),
            field(
                ryo_values::domain::descriptions::constants::REPLACEMENT,
                "Literal replacement text.",
                FieldRequirement::Required,
            ),
            field(
                ryo_values::domain::descriptions::constants::OUTPUT,
                "New output column.",
                FieldRequirement::Required,
            ),
        ],
        &[],
        r#"let example = replace(from="function", text="name", pattern="_", replacement="-", output="label"); emit example subject file key(label);"#,
    )
}
