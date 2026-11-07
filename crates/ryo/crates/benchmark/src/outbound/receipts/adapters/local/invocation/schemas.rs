//! The shape of the raw invocation record kept beside a captured process, and its encoding from the core's document.
use crate::outbound::receipts::errors::ReceiptsError;
use ryo_benchmark_contracts::domain::documents::types::invocation_document::value::{ArgumentText, EnvironmentName, EnvironmentValue, InvocationDocument};
use ryo_benchmark_contracts::domain::documents::types::text::value::{Pin, Signal};
use ryo_benchmark_contracts::domain::documents::types::text::value::YesNo;
use serde::Serialize;
use std::collections::BTreeMap;

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;

/// A record whose process identity was recorded, or one whose identity could not be.
#[derive(Serialize)]
#[serde(untagged)]
enum InvocationSchema {
    /// The process identity was recorded.
    Complete(CompleteSchema),
    /// The process identity could not be recorded.
    Failed(FailedSchema),
}

/// A capture whose process identity was recorded.
#[derive(Serialize)]
struct CompleteSchema {
    actual_request_root: String,
    files: Vec<String>,
    executable_sha256: String,
    settings_sha256: String,
    normalized_settings: serde_json::Value,
    profile_sha256: String,
    tool: String,
    version: String,
    state: String,
    arguments: Vec<String>,
    environment: BTreeMap<String, String>,
    directory: String,
    exit: i64,
    signal: Option<i64>,
    observer_no_deadline: bool,
    source_sha256: Option<String>,
    file_metadata_sha256: Option<String>,
}

/// A capture whose process identity could not be recorded.
#[derive(Serialize)]
struct FailedSchema {
    actual_request_root: String,
    files: Vec<String>,
    arguments: Vec<String>,
    environment: BTreeMap<String, String>,
    directory: String,
    exit: i64,
    signal: Option<i64>,
    observer_no_deadline: bool,
    metadata_error: String,
    source_sha256: Option<String>,
    file_metadata_sha256: Option<String>,
}

fn arguments(values: &[ArgumentText]) -> Vec<String> {
    values.iter().map(|value| value.0.clone()).collect()
}

fn environment(
    values: &BTreeMap<EnvironmentName, EnvironmentValue>,
) -> BTreeMap<String, String> {
    values
        .iter()
        .map(|(name, value)| (name.0.clone(), value.0.clone()))
        .collect()
}

fn signal(value: &Signal) -> Option<i64> {
    match value {
        Signal::Absent => None,
        Signal::Number(number) => Some(number.0),
    }
}

fn pin(value: &Pin) -> Option<String> {
    match value {
        Pin::Absent => None,
        Pin::Sha256(digest) => Some(digest.0.clone()),
    }
}

impl From<&InvocationDocument> for InvocationSchema {
    fn from(document: &InvocationDocument) -> Self {
        match document {
            InvocationDocument::Complete(complete) => Self::Complete(CompleteSchema {
                actual_request_root: complete.actual_request_root.0.clone(),
                files: complete.files.iter().map(|file| file.0.clone()).collect(),
                executable_sha256: complete.executable_sha256.0.clone(),
                settings_sha256: complete.settings_sha256.0.clone(),
                normalized_settings: serde_json::from_str(&complete.normalized_settings.canonical.0)
                    .unwrap_or(serde_json::Value::Null),
                profile_sha256: complete.profile_sha256.0.clone(),
                tool: complete.tool.0.clone(),
                version: complete.version.0.clone(),
                state: complete.state.0.clone(),
                arguments: arguments(&complete.arguments),
                environment: environment(&complete.environment),
                directory: complete.directory.0.clone(),
                exit: complete.exit.0,
                signal: signal(&complete.signal),
                observer_no_deadline: complete.observer_no_deadline == YesNo::Yes,
                source_sha256: pin(&complete.source_sha256),
                file_metadata_sha256: pin(&complete.file_metadata_sha256),
            }),
            InvocationDocument::Failed(failed) => Self::Failed(FailedSchema {
                actual_request_root: failed.actual_request_root.0.clone(),
                files: failed.files.iter().map(|file| file.0.clone()).collect(),
                arguments: arguments(&failed.arguments),
                environment: environment(&failed.environment),
                directory: failed.directory.0.clone(),
                exit: failed.exit.0,
                signal: signal(&failed.signal),
                observer_no_deadline: failed.observer_no_deadline == YesNo::Yes,
                metadata_error: failed.metadata_error.0.clone(),
                source_sha256: pin(&failed.source_sha256),
                file_metadata_sha256: pin(&failed.file_metadata_sha256),
            }),
        }
    }
}

/// Encode one invocation record as the JSON file kept beside its process bytes.
pub(in super::super) fn bytes(document: &InvocationDocument) -> Result<Vec<u8>, ReceiptsError> {
    serde_json::to_vec(&InvocationSchema::from(document))
        .map_err(|error| ReceiptsError::Invalid(error.to_string()))
}
