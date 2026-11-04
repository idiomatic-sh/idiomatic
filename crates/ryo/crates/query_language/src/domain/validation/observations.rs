//! Bound input names inhabit the same explicit source namespace as catalog and hosted inputs.
use ryo_relations::domain::relations::{
    service::relation_named,
    types::{NameRef, Relation},
};
use ryo_values::domain::{
    hosted::types::HostedRelation, observations::types::BoundObservationSchema,
};
use std::collections::BTreeSet;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ObservationInputError {
    #[error("an observation input name is blank")]
    BlankName,
    #[error("observation input `{name}` repeats or shadows an existing source")]
    SourceCollision { name: String },
    #[error("observation input `{input}` has a blank or repeated column `{column}`")]
    Column { input: String, column: String },
}
/// No external declaration can silently replace a catalog/host source during resolution.
pub(crate) fn validate_inputs(
    catalog: &[Relation],
    hosted: &[HostedRelation],
    inputs: &[BoundObservationSchema],
) -> Result<(), ObservationInputError> {
    let mut names = BTreeSet::new();
    for input in inputs {
        let name = &input.name.0;
        if name.trim().is_empty() {
            return Err(ObservationInputError::BlankName);
        }
        if !names.insert(name)
            || relation_named(NameRef(name)).is_some_and(|relation| catalog.contains(&relation))
            || hosted.iter().any(|host| host.name.0 == *name)
        {
            return Err(ObservationInputError::SourceCollision {
                name: name.to_string(),
            });
        }
        let mut columns = BTreeSet::new();
        for column in &input.columns {
            if column.name.0.trim().is_empty() || !columns.insert(&column.name) {
                return Err(ObservationInputError::Column {
                    input: name.to_string(),
                    column: column.name.0.to_string(),
                });
            }
        }
    }
    Ok(())
}
