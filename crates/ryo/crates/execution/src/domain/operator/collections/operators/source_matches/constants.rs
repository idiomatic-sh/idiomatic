//! Public description of physical source occurrence matching.

use crate::domain::describing::constants::{described, field};
use ryo_values::domain::descriptions::types::{FieldRequirement, OperatorDescription};

pub(crate) fn description() -> OperatorDescription {
    described(
        "source-matches",
        "For each per-file input row, append one absolute physical span for each non-overlapping regex occurrence in its checked source span. Input order and duplicates are preserved; zero-width matches use the regex engine's ordered match positions. Absent spans emit no rows; invalid ranges fail before that row emits.",
        vec![
            field("from", "Per-file input rows.", FieldRequirement::Required),
            field(
                "span",
                "Physical source span column to search.",
                FieldRequirement::Required,
            ),
            field(
                "pattern",
                "Literal regular expression.",
                FieldRequirement::Required,
            ),
            field(
                "output",
                "New absolute occurrence span column.",
                FieldRequirement::Required,
            ),
            field(
                "base-line",
                "Optional positive one-based line at the input span's start; requires line-output.",
                FieldRequirement::Optional,
            ),
            field(
                "line-output",
                "Optional output line at each match start; requires base-line.",
                FieldRequirement::Optional,
            ),
        ],
        &[],
        r#"let example = source-matches(from="syntax-tree", span="span", pattern="TODO", output="match_span"); emit example subject file key(match_span);"#,
    )
}
