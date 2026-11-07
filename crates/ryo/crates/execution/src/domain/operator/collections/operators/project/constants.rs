//! Public description and example of this operator.

use crate::domain::describing::constants::{described, field};
use ryo_values::domain::descriptions::constants::{COLUMNS, PROJECT};
use ryo_values::domain::descriptions::types::{FieldRequirement, OperatorDescription};

/// Describe this operator's inputs, output and independent example.
pub(crate) fn description() -> OperatorDescription {
    described(
        PROJECT,
        "Keep named columns and deduplicate equal rows.",
        vec![
            field("from", "Input to project.", FieldRequirement::Required),
            field(
                COLUMNS,
                "Columns retained in output.",
                FieldRequirement::Required,
            ),
        ],
        &[],
        r#"let example = select file, name from "function"; emit example subject file key(name);"#,
    )
}
