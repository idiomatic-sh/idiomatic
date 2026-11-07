//! The raw invocation record kept beside a captured process.
use crate::outbound::inventory::adapters::local::constants;
use crate::outbound::inventory::adapters::local::types::InvocationBytes;
use ryo_benchmark_contracts::domain::documents::types::invocation_document::value::{ArgumentText, CompleteInvocation, EnvironmentName, EnvironmentValue, FailedInvocation, InvocationDocument, NormalizedSettings};
use ryo_benchmark_contracts::domain::documents::types::text::value::{Pin, Signal};
use ryo_benchmark_contracts::domain::documents::types::text::value::ExitStatus;
use ryo_benchmark_contracts::domain::documents::types::text::value::{CanonicalJson, PathText, ReasonText, Sha256Text, YesNo};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use serde::Deserialize;
use std::collections::BTreeMap;

/// One raw invocation record kept beside a captured process; a surprise field is refused.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InvocationSchema {
    actual_request_root: String,
    files: Vec<String>,
    arguments: Vec<String>,
    environment: BTreeMap<String, String>,
    directory: String,
    exit: i64,
    signal: Option<i64>,
    observer_no_deadline: bool,
    metadata_error: Option<String>,
    source_sha256: Option<String>,
    file_metadata_sha256: Option<String>,
    executable_sha256: Option<String>,
    settings_sha256: Option<String>,
    normalized_settings: Option<serde_json::Value>,
    profile_sha256: Option<String>,
    tool: Option<String>,
    version: Option<String>,
    state: Option<String>,
}

/// The parts of the normalized settings that raw metadata re-derives.
#[derive(Debug, Deserialize)]
struct SettingsSchema {
    arguments: Vec<String>,
    environment: BTreeMap<String, String>,
    directory: String,
}

fn argument_texts(values: Vec<String>) -> Vec<ArgumentText> {
    values.into_iter().map(ArgumentText).collect()
}

fn environment_texts(
    values: BTreeMap<String, String>,
) -> BTreeMap<EnvironmentName, EnvironmentValue> {
    values
        .into_iter()
        .map(|(name, value)| (EnvironmentName(name), EnvironmentValue(value)))
        .collect()
}

fn pin(value: Option<String>) -> Pin {
    value.map_or(Pin::Absent, |digest| Pin::Sha256(Sha256Text(digest)))
}

impl TryFrom<InvocationSchema> for InvocationDocument {
    type Error = Error;

    fn try_from(wire: InvocationSchema) -> Result<Self, Error> {
        let signal = wire
            .signal
            .map_or(Signal::Absent, |number| Signal::Number(ExitStatus(number)));
        let files = wire.files.into_iter().map(PathText).collect();
        let arguments = argument_texts(wire.arguments);
        let environment = environment_texts(wire.environment);
        if let Some(error) = wire.metadata_error {
            return Ok(Self::Failed(FailedInvocation {
                actual_request_root: PathText(wire.actual_request_root),
                files,
                arguments,
                environment,
                directory: PathText(wire.directory),
                exit: ExitStatus(wire.exit),
                signal,
                observer_no_deadline: YesNo::from(wire.observer_no_deadline),
                metadata_error: ReasonText(error),
                source_sha256: pin(wire.source_sha256),
                file_metadata_sha256: pin(wire.file_metadata_sha256),
            }));
        }
        let (
            Some(executable),
            Some(settings),
            Some(normalized),
            Some(profile),
            Some(tool),
            Some(version),
            Some(state),
        ) = (
            wire.executable_sha256,
            wire.settings_sha256,
            wire.normalized_settings,
            wire.profile_sha256,
            wire.tool,
            wire.version,
            wire.state,
        )
        else {
            return Err(Error::Invalid(constants::INVOCATION_LACKS_PROCESS_IDENTITY.into()));
        };
        let parts: SettingsSchema = serde_json::from_value(normalized.clone())
            .map_err(|error| Error::Json(Box::new(error)))?;
        Ok(Self::Complete(CompleteInvocation {
            actual_request_root: PathText(wire.actual_request_root),
            files,
            executable_sha256: Sha256Text(executable),
            settings_sha256: Sha256Text(settings),
            normalized_settings: NormalizedSettings {
                arguments: argument_texts(parts.arguments),
                environment: environment_texts(parts.environment),
                directory: PathText(parts.directory),
                canonical: CanonicalJson(normalized.to_string()),
            },
            profile_sha256: Sha256Text(profile),
            tool: ReasonText(tool),
            version: ReasonText(version),
            state: PathText(state),
            arguments,
            environment,
            directory: PathText(wire.directory),
            exit: ExitStatus(wire.exit),
            signal,
            observer_no_deadline: YesNo::from(wire.observer_no_deadline),
            source_sha256: pin(wire.source_sha256),
            file_metadata_sha256: pin(wire.file_metadata_sha256),
        }))
    }
}


impl TryFrom<InvocationBytes<'_>> for InvocationDocument {
    type Error = Error;

    fn try_from(bytes: InvocationBytes<'_>) -> Result<Self, Error> {
        serde_json::from_slice::<InvocationSchema>(&bytes.0.0)
            .map_err(|error| Error::Json(Box::new(error)))?
            .try_into()
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
