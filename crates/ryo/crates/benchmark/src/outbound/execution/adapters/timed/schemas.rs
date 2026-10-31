//! Private wire forms preserve native OS strings and exact resource units.
use crate::domain::measurements::types::CapturedBytes as CollectorMessage;
use crate::domain::{
    diagnostics::errors::DiagnosticsError as Error,
    measurements::types::{
        CapturedBytes, Invocation, ProcessExitCode, ProcessOutput, ResidentBytes,
        ResourceObservation, Resources,
    },
};
use serde::{Deserialize, Serialize};
use std::{ffi::OsString, time::Duration};
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EnvironmentSchema {
    name: OsString,
    value: OsString,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InvocationSchema {
    executable: OsString,
    arguments: Vec<OsString>,
    environment: Vec<EnvironmentSchema>,
    directory: OsString,
}
impl From<&Invocation> for InvocationSchema {
    fn from(call: &Invocation) -> Self {
        Self {
            executable: call.executable.clone().into_os_string(),
            arguments: call.arguments.clone(),
            environment: call
                .environment
                .iter()
                .map(|(name, value)| EnvironmentSchema {
                    name: name.clone(),
                    value: value.clone(),
                })
                .collect(),
            directory: call.directory.clone().into_os_string(),
        }
    }
}
impl From<InvocationSchema> for Invocation {
    fn from(call: InvocationSchema) -> Self {
        Self {
            executable: call.executable.into(),
            arguments: call.arguments,
            environment: call
                .environment
                .into_iter()
                .map(|row| (row.name, row.value))
                .collect(),
            directory: call.directory.into(),
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservationSchema {
    status: i32,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    wall: Duration,
    cpu: Duration,
    rss: u64,
}
impl From<ObservationSchema> for ProcessOutput {
    fn from(output: ObservationSchema) -> Self {
        Self {
            status: ProcessExitCode(output.status),
            stdout: CapturedBytes(output.stdout),
            stderr: CapturedBytes(output.stderr),
            wall: output.wall,
            resources: ResourceObservation::Collected(Resources {
                cpu: output.cpu,
                peak_rss_bytes: ResidentBytes(output.rss),
            }),
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "result", deny_unknown_fields)]
enum ResponseSchema {
    Observed { output: ObservationSchema },
    LaunchFailed { errno: i32 },
    Failed { message: String },
}
impl From<ResponseSchema> for Result<ProcessOutput, Error> {
    fn from(wire: ResponseSchema) -> Self {
        match wire {
            ResponseSchema::Observed { output } => Ok(output.into()),
            ResponseSchema::LaunchFailed { errno } => {
                Err(std::io::Error::from_raw_os_error(errno).into())
            }
            ResponseSchema::Failed { message } => Err(Error::Invalid(message)),
        }
    }
}
pub(super) fn request(call: &Invocation) -> Result<CollectorMessage, Error> {
    serde_json::to_vec(&InvocationSchema::from(call))
        .map(CollectorMessage)
        .map_err(|error| Error::Json(Box::new(error)))
}
pub(super) fn observation(
    message: &CollectorMessage,
) -> Result<Result<ProcessOutput, Error>, Error> {
    serde_json::from_slice::<ResponseSchema>(&message.0)
        .map(Into::into)
        .map_err(|error| Error::Json(Box::new(error)))
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
