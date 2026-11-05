//! Public description and example of this operator.

use crate::domain::describing::constants::{described, field};
use ryo_values::domain::descriptions::types::{FieldRequirement, OperatorDescription};

/// Describe this operator's inputs, output and independent example.
pub(crate) fn description() -> OperatorDescription {
    described(
        "ordered-match",
        "Consume the earliest later right row for each ordered left row in an equal-key group.",
        vec![
            field("left", "Left input.", FieldRequirement::Required),
            field("right", "Right input.", FieldRequirement::Required),
            field(
                ryo_values::domain::descriptions::constants::LEFT_ON,
                "Left equality keys.",
                FieldRequirement::Required,
            ),
            field(
                ryo_values::domain::descriptions::constants::RIGHT_ON,
                "Right equality keys.",
                FieldRequirement::Required,
            ),
            field(
                "left-order",
                "Left numeric order.",
                FieldRequirement::Required,
            ),
            field(
                "right-order",
                "Right numeric order.",
                FieldRequirement::Required,
            ),
            field(
                ryo_values::domain::descriptions::constants::OUTPUT,
                "Named columns with explicit input sources.",
                FieldRequirement::Required,
            ),
        ],
        &[],
        r#"let requests = extend(from="function", values={sequence: 1}); let replies = extend(from="function", values={sequence: 2}); let example = ordered-match(left=requests, right=replies, left-on=[file,name], right-on=[file,name], left-order="sequence", right-order="sequence", output={file: [left,file], name: [right,name]}); emit example subject file key(name); "#,
    )
}
