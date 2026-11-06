//! Process certificates the case boundary admits: wire shapes map to contract values here.
use crate::outbound::suite::adapters::local::constants;
use crate::outbound::suite::types::process::{
    OutputPartWire, PathEncodingWire, ProcessArtifactWire, ProcessFailureWire,
    ProcessObservationWire,
};
use ryo_benchmark_contracts::domain::artifacts::types::{
    digest::value::ArtifactDigest, relative_artifact_path::value::RelativeArtifactPath,
};
use ryo_benchmark_contracts::domain::process::types::{
    OutputPart, PathEncoding, ProcessArtifact, ProcessFailure, ProcessObservation,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;


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

impl TryFrom<ProcessArtifactWire> for ProcessArtifact {
    type Error = Error;

    fn try_from(wire: ProcessArtifactWire) -> Result<Self, Error> {
        let _path = RelativeArtifactPath::try_from(wire.path.as_path())?;
        Ok(Self {
            path: wire.path,
            sha256: identity_digest(wire.sha256)?,
        })
    }
}

impl TryFrom<OutputPartWire> for OutputPart {
    type Error = Error;

    fn try_from(wire: OutputPartWire) -> Result<Self, Error> {
        match wire {
            OutputPartWire::Literal { value } => Ok(Self::Literal(value)),
            OutputPartWire::InputPath { file, encoding } => {
                let _file = RelativeArtifactPath::try_from(file.as_path())?;
                Ok(Self::InputPath {
                    file,
                    encoding: match encoding {
                        PathEncodingWire::Utf8Bytes => PathEncoding::Utf8Bytes,
                        PathEncodingWire::JsonStringContent => PathEncoding::JsonStringContent,
                    },
                })
            }
        }
    }
}

impl TryFrom<ProcessObservationWire> for ProcessObservation {
    type Error = Error;

    fn try_from(wire: ProcessObservationWire) -> Result<Self, Error> {
        Ok(Self {
            executable_sha256: identity_digest(wire.executable_sha256)?,
            invocation: wire.invocation.try_into()?,
            stdout: wire.stdout.try_into()?,
            stderr: wire.stderr.try_into()?,
        })
    }
}

/// Decode only explicit tagged process expectations and preserve every raw artifact identity.
impl TryFrom<ProcessFailureWire> for ProcessFailure {
    type Error = Error;

    fn try_from(wire: ProcessFailureWire) -> Result<Self, Error> {
        let ProcessFailureWire::ProcessFailure {
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
            exit,
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
