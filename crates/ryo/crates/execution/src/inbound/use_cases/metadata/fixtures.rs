//! The shared helper of the metadata case table.

/// Retain the former operator-service example/catalog assertion through public descriptors.
pub fn example_unknown_relations(example: &str) -> usize {
    use ryo_relations::domain::labels::types::NameRef;
    use ryo_relations::domain::relations::service::relation_named;
    let fields = [
        "from",
        "from=",
        "left=",
        "right=",
        "edges=",
        "relation=",
        "domain=",
    ];
    example
        .split('"')
        .skip(1)
        .step_by(2)
        .filter(|quoted| {
            let before = example
                .split(&format!("\"{quoted}\""))
                .next()
                .unwrap_or_default()
                .trim_end();
            fields.iter().any(|field| before.ends_with(field))
                && relation_named(NameRef(quoted)).is_none()
        })
        .count()
}

pub mod contract;
