//! Private wire forms preserve native OS strings and exact resource units.
use crate::domain::{
    diagnostics::errors::DiagnosticsError as Error,
    measurements::types::{
        CapturedBytes, Invocation, ProcessExitCode, ProcessOutput, ResidentBytes,
        ResourceObservation, Resources,
    },
};
use crate::outbound::collection::types::CollectorMessage;
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
fn observation_wire(output: ProcessOutput) -> Result<ObservationSchema, Error> {
    let ResourceObservation::Collected(resources) = output.resources else {
        return Err(Error::Invalid("collector returned no resources".into()));
    };
    Ok(ObservationSchema {
        status: output.status.0,
        stdout: output.stdout.0,
        stderr: output.stderr.0,
        wall: output.wall,
        cpu: resources.cpu,
        rss: resources.peak_rss_bytes.0,
    })
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
fn response_wire(result: Result<ProcessOutput, Error>) -> Result<ResponseSchema, Error> {
    Ok(match result {
        Ok(output) => ResponseSchema::Observed {
            output: observation_wire(output)?,
        },
        Err(Error::Io(error)) => match error.raw_os_error() {
            Some(errno) => ResponseSchema::LaunchFailed { errno },
            None => ResponseSchema::Failed {
                message: error.to_string(),
            },
        },
        Err(error) => ResponseSchema::Failed {
            message: error.to_string(),
        },
    })
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
pub(super) fn invocation(message: &CollectorMessage) -> Result<Invocation, Error> {
    serde_json::from_slice::<InvocationSchema>(&message.0)
        .map(Into::into)
        .map_err(|error| Error::Json(Box::new(error)))
}
pub(super) fn response(result: Result<ProcessOutput, Error>) -> Result<CollectorMessage, Error> {
    serde_json::to_vec(&response_wire(result)?)
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
