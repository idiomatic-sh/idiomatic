//! The shape of the process record a receipt row carries, and its conversions from and into the core's record.
use ryo_benchmark_contracts::domain::documents::types::observation_document::value::{
    ProcessArtifacts, ProcessArtifactsRecord, ProcessRecord, RecordedReason,
};
use ryo_benchmark_contracts::domain::documents::types::reference_discrepancy::value::{
    ArtifactPointer, ExpectationKind,
};
use ryo_benchmark_contracts::domain::documents::types::text::value::{
    ExitStatus, PathText, Pin, ReasonText, Sha256Text, Signal, YesNo,
};
use serde::{Deserialize, Serialize};

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;

/// What kind of expectation the reference side was validated against.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(in super::super) enum KindSchema {
    Diagnostics,
    ProcessFailure,
}

/// A raw artifact a row points at.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PointerSchema {
    path: String,
    sha256: String,
}

/// The raw artifacts of a process-failure reference.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ProcessArtifactsSchema {
    stdout: PointerSchema,
    stderr: PointerSchema,
    invocation: PointerSchema,
}

/// The reference process a side observed instead of diagnostics.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(in super::super) struct ProcessRowSchema {
    kind: KindSchema,
    exit: i64,
    signal: Option<i64>,
    actual_request_root: String,
    observer_no_deadline: bool,
    executable_sha256: Option<String>,
    settings_sha256: Option<String>,
    metadata_error: Option<String>,
    template_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    artifacts: Option<ProcessArtifactsSchema>,
}

/// An identity a record may leave out.
pub(in super::super) fn pin(value: Option<String>) -> Pin {
    value.map_or(Pin::Absent, |digest| Pin::Sha256(Sha256Text(digest)))
}

impl From<KindSchema> for ExpectationKind {
    fn from(schema: KindSchema) -> Self {
        match schema {
            KindSchema::Diagnostics => Self::Diagnostics,
            KindSchema::ProcessFailure => Self::ProcessFailure,
        }
    }
}

impl From<PointerSchema> for ArtifactPointer {
    fn from(schema: PointerSchema) -> Self {
        Self {
            path: PathText(schema.path),
            sha256: Sha256Text(schema.sha256),
        }
    }
}

impl From<ProcessRowSchema> for ProcessRecord {
    fn from(schema: ProcessRowSchema) -> Self {
        Self {
            kind: schema.kind.into(),
            exit: ExitStatus(schema.exit),
            signal: schema
                .signal
                .map_or(Signal::Absent, |number| Signal::Number(ExitStatus(number))),
            actual_request_root: PathText(schema.actual_request_root),
            observer_no_deadline: YesNo::from(schema.observer_no_deadline),
            executable_sha256: pin(schema.executable_sha256),
            settings_sha256: pin(schema.settings_sha256),
            metadata_error: schema
                .metadata_error
                .map_or(RecordedReason::Absent, |reason| {
                    RecordedReason::Stated(ReasonText(reason))
                }),
            template_sha256: pin(schema.template_sha256),
            artifacts: schema
                .artifacts
                .map_or(ProcessArtifactsRecord::Unattached, |artifacts| {
                    ProcessArtifactsRecord::Attached(ProcessArtifacts {
                        stdout: artifacts.stdout.into(),
                        stderr: artifacts.stderr.into(),
                        invocation: artifacts.invocation.into(),
                    })
                }),
        }
    }
}

/// An identity as a record writes it: none when the record leaves it out.
fn pin_text(value: &Pin) -> Option<String> {
    match value {
        Pin::Sha256(digest) => Some(digest.0.clone()),
        Pin::Absent => None,
    }
}

impl From<ExpectationKind> for KindSchema {
    fn from(kind: ExpectationKind) -> Self {
        match kind {
            ExpectationKind::Diagnostics => Self::Diagnostics,
            ExpectationKind::ProcessFailure => Self::ProcessFailure,
        }
    }
}

impl From<&ArtifactPointer> for PointerSchema {
    fn from(pointer: &ArtifactPointer) -> Self {
        Self {
            path: pointer.path.0.clone(),
            sha256: pointer.sha256.0.clone(),
        }
    }
}

impl From<&ProcessRecord> for ProcessRowSchema {
    fn from(process: &ProcessRecord) -> Self {
        Self {
            kind: process.kind.into(),
            exit: process.exit.0,
            signal: match process.signal {
                Signal::Number(number) => Some(number.0),
                Signal::Absent => None,
            },
            actual_request_root: process.actual_request_root.0.clone(),
            observer_no_deadline: process.observer_no_deadline == YesNo::Yes,
            executable_sha256: pin_text(&process.executable_sha256),
            settings_sha256: pin_text(&process.settings_sha256),
            metadata_error: match &process.metadata_error {
                RecordedReason::Stated(reason) => Some(reason.0.clone()),
                RecordedReason::Absent => None,
            },
            template_sha256: pin_text(&process.template_sha256),
            artifacts: match &process.artifacts {
                ProcessArtifactsRecord::Attached(artifacts) => Some(ProcessArtifactsSchema {
                    stdout: (&artifacts.stdout).into(),
                    stderr: (&artifacts.stderr).into(),
                    invocation: (&artifacts.invocation).into(),
                }),
                ProcessArtifactsRecord::Unattached => None,
            },
        }
    }
}
