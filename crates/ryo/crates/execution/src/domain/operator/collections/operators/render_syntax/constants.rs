//! Public description of request-driven semantic syntax rendering.

use crate::domain::describing::constants::{described, field};
use ryo_values::domain::descriptions::constants::OUTPUT;
use ryo_values::domain::descriptions::types::{FieldRequirement, OperatorDescription};

pub(crate) fn description() -> OperatorDescription {
    described(
        "render-syntax",
        "Append exact provider-rendered syntax for an already selected node in a declared embedding context. Unsupported valid syntax makes the consuming rule incomplete.",
        vec![
            field(
                "from",
                "Candidate rows from this file.",
                FieldRequirement::Required,
            ),
            field("node", "Node column to render.", FieldRequirement::Required),
            field(
                "context",
                "Embedding role: assignment-target, attribute-receiver, or positional-argument.",
                FieldRequirement::Required,
            ),
            field(OUTPUT, "New exact text column.", FieldRequirement::Required),
        ],
        &[],
        r#"let example = render-syntax(from="syntax-tree", node="id", context="assignment-target", output="rendered"); emit example subject file key(id);"#,
    )
}

/// What a render demand says when the file admitted no semantic session.
pub(super) const NO_SEMANTIC_SESSION: &str = "no file semantic session was admitted";
