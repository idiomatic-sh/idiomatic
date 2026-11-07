//! The shape of the selection file this adapter reads: one JSON list of the names it selects.
use crate::outbound::suite::types::SelectedName;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use serde::Deserialize;

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;

/// The names a selection file lists.
#[derive(Deserialize)]
struct SelectionSchema(Vec<String>);

/// Decode a selection file into the names it lists.
pub(in super::super) fn names(bytes: &[u8]) -> Result<Vec<SelectedName>, Error> {
    let schema: SelectionSchema =
        serde_json::from_slice(bytes).map_err(|error| Error::Invalid(error.to_string()))?;
    Ok(schema.0.into_iter().map(SelectedName).collect())
}
