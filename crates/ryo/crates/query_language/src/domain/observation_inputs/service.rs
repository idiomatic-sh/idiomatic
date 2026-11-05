//! Bound input names inhabit the same explicit source namespace as catalog and hosted inputs.
use super::errors::ObservationInputsError;
use ryo_relations::domain::relations::{
    service::relation_named,
    types::{NameRef, Relation},
};
use ryo_values::domain::{
    hosted::types::HostedRelation, observations::types::BoundObservationSchema,
};
use std::collections::BTreeSet;

/// No external declaration can silently replace a catalog/host source during resolution.
pub(crate) fn validate_inputs(
    catalog: &[Relation],
    hosted: &[HostedRelation],
    inputs: &[BoundObservationSchema],
) -> Result<(), ObservationInputsError> {
    let mut names = BTreeSet::new();
    for input in inputs {
        let name = &input.name.0;
        if name.trim().is_empty() {
            return Err(ObservationInputsError::BlankName);
        }
        if !names.insert(name)
            || relation_named(NameRef(name)).is_some_and(|relation| catalog.contains(&relation))
            || hosted.iter().any(|host| host.name.0 == *name)
        {
            return Err(ObservationInputsError::SourceCollision {
                name: input.name.clone(),
            });
        }
        let mut columns = BTreeSet::new();
        for column in &input.columns {
            if column.name.0.trim().is_empty() || !columns.insert(&column.name) {
                return Err(ObservationInputsError::Column {
                    input: input.name.clone(),
                    column: column.name.clone(),
                });
            }
        }
    }
    Ok(())
}
