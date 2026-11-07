//! The ordered DFS call's public description.
use crate::domain::describing::constants::{described, field, DESTINATION};
use ryo_values::domain::descriptions::constants::OUTPUT;
use ryo_values::domain::descriptions::types::{FieldRequirement, OperatorDescription};
/// Describe independent ordered graph walks and their emitted edge visits.
pub(crate) fn description() -> OperatorDescription {
    described(
        "ordered-dfs",
        "Visit outgoing edges in numeric order, recursively depth first for each distinct partition/root.",
        vec![
            field("from", "Root rows.", FieldRequirement::Required),
            field("root", "Root entity column.", FieldRequirement::Required),
            field("edges", "Directed edge rows.", FieldRequirement::Required),
            field("source", "Edge source column.", FieldRequirement::Required),
            field(
                DESTINATION,
                "Edge destination column.",
                FieldRequirement::Required,
            ),
            field(
                "order",
                "Numeric outgoing-edge order; ties are rejected.",
                FieldRequirement::Required,
            ),
            field(
                "edge",
                "Edge identity column retained on every visit.",
                FieldRequirement::Required,
            ),
            field(
                "keys",
                "Matching partition columns of roots and edges, in output order; omitted means one graph.",
                FieldRequirement::Optional,
            ),
            field(
                OUTPUT,
                "Name of the zero-based visit ordinal column.",
                FieldRequirement::Required,
            ),
        ],
        &[],
        r#"let roots = select id as root, file from "function"; let edges = select parent as source, child as destination, ordinal as position, child as base, file from "syntax-child"; let example = ordered-dfs(from=roots,root=root,edges=edges,source=source,destination=destination,order=position,edge=base,keys=[file],output=visit); emit example subject file key(base,visit);"#,
    )
}
