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
/// One native environment override in the collector protocol.
struct EnvironmentSchema {
    /// Environment variable name, preserving native OS encoding.
    name: OsString,
    /// Environment variable value, preserving native OS encoding.
    value: OsString,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Serializable child invocation with native OS strings.
struct InvocationSchema {
    /// Exact executable path to launch.
    executable: OsString,
    /// Ordered argument vector without shell interpretation.
    arguments: Vec<OsString>,
    /// Explicit environment overrides for the child.
    environment: Vec<EnvironmentSchema>,
    /// Child working directory.
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
/// Completed child observation with exact resource units.
struct ObservationSchema {
    /// Unix termination signal, independent of numeric exit status.
    #[serde(default)]
    signal: Option<i32>,
    /// Child exit status, including the agreed signal encoding.
    status: i32,
    /// Captured standard output bytes without text decoding.
    stdout: Vec<u8>,
    /// Captured standard error bytes without text decoding.
    stderr: Vec<u8>,
    /// Elapsed child execution time.
    wall: Duration,
    /// Sum of child user and system CPU time.
    cpu: Duration,
    /// Child peak resident memory in bytes.
    rss: u64,
}
impl From<ObservationSchema> for ProcessOutput {
    fn from(output: ObservationSchema) -> Self {
        Self {
            signal: output.signal,
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
/// Collector response separating launch failures from child exit statuses.
enum ResponseSchema {
    /// Child launched and completed with measured resources.
    Observed {
        /// Captured output and resource observation.
        output: ObservationSchema,
    },
    /// The child could not be launched.
    LaunchFailed {
        /// Original operating-system launch error code.
        errno: i32,
    },
    /// Collection failed without an operating-system launch error.
    Failed {
        /// Diagnostic describing the collection failure.
        message: String,
    },
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
/// Serialize the exact invocation for the collector boundary.
pub(super) fn request(call: &Invocation) -> Result<CollectorMessage, Error> {
    serde_json::to_vec(&InvocationSchema::from(call))
        .map(CollectorMessage)
        .map_err(|error| Error::Json(Box::new(error)))
}
/// Decode the response while preserving launch failure identity.
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
