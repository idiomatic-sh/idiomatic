//! Public description of physical rendered line-width admission.

use crate::domain::describing::constants::{described, field};
use ryo_values::domain::descriptions::types::{FieldRequirement, OperatorDescription};

pub(crate) fn description() -> OperatorDescription {
    described(
        "fits-rendered",
        "Keep rendered candidates whose physical display lines fit a declared maximum, including the original source prefix on the first line and tab stops.",
        vec![
            field("from", "Rendered candidate rows from this file.", FieldRequirement::Required),
            field("text", "Exact rendered candidate text column.", FieldRequirement::Required),
            field("span", "Original physical statement span.", FieldRequirement::Required),
            field("max-line", "Maximum display columns.", FieldRequirement::Required),
            field("tab-width", "Physical tab stop width.", FieldRequirement::Required),
        ],
        &[],
        r#"let example = fits-rendered(from="syntax-tree", text="name", span="span", max-line=88, tab-width=8); emit example subject file key(id);"#,
    )
}
