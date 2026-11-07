//! Process certificates the case boundary admits: wire shapes map to contract values here.
use crate::outbound::suite::adapters::local::constants;
use ryo_benchmark_contracts::domain::artifacts::types::{
    digest::value::ArtifactDigest, relative_artifact_path::value::RelativeArtifactPath,
};
use ryo_benchmark_contracts::domain::process::types::{
    OutputPart, PathEncoding, ProcessArtifact, ProcessFailure, ProcessObservation,
};
use ryo_benchmark_contracts::domain::measurements::types::ProcessExitCode;
use ryo_benchmark_contracts::domain::process::types::OutputText;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use serde::Deserialize;
use std::path::PathBuf;

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;


/// Only this explicit variant can request process-failure comparison.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(in super::super) enum ProcessFailureSchema {
    /// A completed numeric failure with exactly pinned byte streams.
    ProcessFailure {
        /// Exact nonzero numeric status, never a signal.
        exit: i32,
        /// Whole output template.
        stdout: Vec<OutputPartSchema>,
        /// Whole error template.
        stderr: Vec<OutputPartSchema>,
        /// Two captures for each exact supported executable.
        observations: Vec<ProcessObservationSchema>,
    },
}
/// Exact byte spelling or a declared source path in one explicit encoding.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
enum OutputPartSchema {
    /// Unmodified UTF8 bytes, including whitespace.
    Literal { value: String },
    /// Authoritative invocation root joined to a declared input.
    InputPath {
        file: PathBuf,
        encoding: PathEncodingSchema,
    },
}
/// Output path encoding, without inferred normalization.
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum PathEncodingSchema {
    /// Exact UTF8 bytes.
    Utf8Bytes,
    /// Canonical JSON string content, without surrounding quotes.
    JsonStringContent,
}
/// A raw file stored separately from independent semantic evidence.
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct ProcessArtifactSchema {
    /// Explicit profile-relative path.
    path: PathBuf,
    /// Exact raw byte SHA256.
    sha256: String,
}
/// Independent captured observation grouped by exact executable digest.
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct ProcessObservationSchema {
    /// No cross-architecture substitution is accepted.
    executable_sha256: String,
    /// Captured root/normalized settings/exit/source identity.
    invocation: ProcessArtifactSchema,
    /// Actual raw standard output.
    stdout: ProcessArtifactSchema,
    /// Actual raw standard error.
    stderr: ProcessArtifactSchema,
}


/// An identity digest is exactly 64 lowercase hexadecimal digits.
fn identity_digest(value: String) -> Result<ArtifactDigest, Error> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(Error::Invalid(
            constants::DISCREPANCY_REQUIRES_LOWERCASE_SHA256_IDENTITIES.into(),
        ));
    }
    Ok(ArtifactDigest(value))
}

impl TryFrom<ProcessArtifactSchema> for ProcessArtifact {
    type Error = Error;

    fn try_from(wire: ProcessArtifactSchema) -> Result<Self, Error> {
        let _path = RelativeArtifactPath::try_from(wire.path.as_path())?;
        Ok(Self {
            path: wire.path,
            sha256: identity_digest(wire.sha256)?,
        })
    }
}

impl TryFrom<OutputPartSchema> for OutputPart {
    type Error = Error;

    fn try_from(wire: OutputPartSchema) -> Result<Self, Error> {
        match wire {
            OutputPartSchema::Literal { value } => Ok(Self::Literal(ryo_benchmark_contracts::domain::process::types::OutputText(value))),
            OutputPartSchema::InputPath { file, encoding } => {
                let _file = RelativeArtifactPath::try_from(file.as_path())?;
                Ok(Self::InputPath {
                    file,
                    encoding: match encoding {
                        PathEncodingSchema::Utf8Bytes => PathEncoding::Utf8Bytes,
                        PathEncodingSchema::JsonStringContent => PathEncoding::JsonStringContent,
                    },
                })
            }
        }
    }
}

impl TryFrom<ProcessObservationSchema> for ProcessObservation {
    type Error = Error;

    fn try_from(wire: ProcessObservationSchema) -> Result<Self, Error> {
        Ok(Self {
            executable_sha256: identity_digest(wire.executable_sha256)?,
            invocation: wire.invocation.try_into()?,
            stdout: wire.stdout.try_into()?,
            stderr: wire.stderr.try_into()?,
        })
    }
}

/// Decode only explicit tagged process expectations and preserve every raw artifact identity.
impl TryFrom<ProcessFailureSchema> for ProcessFailure {
    type Error = Error;

    fn try_from(wire: ProcessFailureSchema) -> Result<Self, Error> {
        let ProcessFailureSchema::ProcessFailure {
            exit,
            stdout,
            stderr,
            observations,
        } = wire;
        if exit <= 0 {
            return Err(Error::Invalid(
                constants::PROCESS_FAILURE_REQUIRES_A_POSITIVE_NUMERIC_EXIT.into(),
            ));
        }
        Ok(Self {
            exit: ryo_benchmark_contracts::domain::measurements::types::ProcessExitCode(exit),
            stdout: stdout
                .into_iter()
                .map(OutputPart::try_from)
                .collect::<Result<_, _>>()?,
            stderr: stderr
                .into_iter()
                .map(OutputPart::try_from)
                .collect::<Result<_, _>>()?,
            observations: observations
                .into_iter()
                .map(ProcessObservation::try_from)
                .collect::<Result<_, _>>()?,
        })
    }
}
