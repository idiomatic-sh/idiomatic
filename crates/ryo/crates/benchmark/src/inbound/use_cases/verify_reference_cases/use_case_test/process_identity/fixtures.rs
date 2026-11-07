//! Raw invocation records, registered documents and the byte streams a fake process observes.
use ryo_benchmark_contracts::domain::documents::types::text::value::YesNo;
use crate::outbound::{
    inventory::{
        interface::InventoryPort as _,
        types::{CompleteInvocation, InvocationDocument, Pin, Registration, Signal, case_document::CaseDocument},
    },
};
use ryo_benchmark_contracts::domain::measurements::types::{
    CapturedBytes, ExecutionProfile, Invocation, ProcessExitCode, ProcessOutput,
    ResourceObservation, TerminationSignal,
};
use crate::outbound::execution::interface::ExecutionPort;
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn pin_text(pin: &Pin) -> Value {
    match pin {
        Pin::Absent => Value::Null,
        Pin::Sha256(digest) => json!(digest.0),
    }
}

/// The raw invocation record file a capture keeps beside its process bytes.
pub(crate) fn invocation_bytes(invocation: &InvocationDocument) -> Vec<u8> {
    let argument_texts = |values: &[ryo_benchmark_contracts::domain::documents::types::invocation_document::value::ArgumentText]| {
        values.iter().map(|value| value.0.clone()).collect::<Vec<_>>()
    };
    let environment_texts = |values: &BTreeMap<
        ryo_benchmark_contracts::domain::documents::types::invocation_document::value::EnvironmentName,
        ryo_benchmark_contracts::domain::documents::types::invocation_document::value::EnvironmentValue,
    >| {
        values
            .iter()
            .map(|(name, value)| (name.0.clone(), value.0.clone()))
            .collect::<BTreeMap<_, _>>()
    };
    let signal = |signal: &Signal| match signal {
        Signal::Absent => Value::Null,
        Signal::Number(number) => json!(number.0),
    };
    let record = match invocation {
        InvocationDocument::Complete(complete) => json!({
            "actual_request_root": complete.actual_request_root.0,
            "files": complete.files.iter().map(|file| file.0.clone()).collect::<Vec<_>>(),
            "executable_sha256": complete.executable_sha256.0,
            "settings_sha256": complete.settings_sha256.0,
            "normalized_settings": serde_json::from_str::<Value>(&complete.normalized_settings.canonical.0).unwrap(),
            "profile_sha256": complete.profile_sha256.0,
            "tool": complete.tool.0, "version": complete.version.0, "state": complete.state.0,
            "arguments": argument_texts(&complete.arguments),
            "environment": environment_texts(&complete.environment),
            "directory": complete.directory.0, "exit": complete.exit.0,
            "signal": signal(&complete.signal),
            "observer_no_deadline": complete.observer_no_deadline == YesNo::Yes,
            "source_sha256": pin_text(&complete.source_sha256),
            "file_metadata_sha256": pin_text(&complete.file_metadata_sha256),
        }),
        InvocationDocument::Failed(failed) => json!({
            "actual_request_root": failed.actual_request_root.0,
            "files": failed.files.iter().map(|file| file.0.clone()).collect::<Vec<_>>(),
            "arguments": argument_texts(&failed.arguments),
            "environment": environment_texts(&failed.environment),
            "directory": failed.directory.0, "exit": failed.exit.0,
            "signal": signal(&failed.signal),
            "observer_no_deadline": failed.observer_no_deadline == YesNo::Yes,
            "metadata_error": failed.metadata_error.0,
            "source_sha256": pin_text(&failed.source_sha256),
            "file_metadata_sha256": pin_text(&failed.file_metadata_sha256),
        }),
    };
    serde_json::to_vec(&record).unwrap()
}

/// The complete variant of a capture whose process identity was recorded.
pub(crate) fn complete_of(invocation: &mut InvocationDocument) -> &mut CompleteInvocation {
    match invocation {
        InvocationDocument::Complete(complete) => complete,
        InvocationDocument::Failed(_) => unreachable!("fixture capture records its identity"),
    }
}
/// Read a registration the way the offline command does: through the adapter's schema.
pub(crate) fn registration_of(case: &Value) -> Registration {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("registration.json");
    std::fs::write(
        &path,
        json!({"authority":{"kind":"receipt-only"},"provider":"toy","defaults":["A001"],"rules":[{"rule_id":"A001","definition":null,"modules":{},"artifacts":{},"command":null,"reason":null,"cases":[case],"instance":null}]}).to_string(),
    )
    .unwrap();
    crate::outbound::inventory::factory::local()
        .load(&path)
        .unwrap()
}

/// One registered case read through the adapter's schema.
pub(crate) fn document_of(case: &Value) -> CaseDocument {
    registration_of(case).rules.remove(0).cases.remove(0)
}

/// Different observed byte streams, independent of any certificate matching implementation.
pub(crate) struct Streams {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}
impl ExecutionPort for Streams {
    fn profile(&self) -> ExecutionProfile {
        ExecutionProfile::Wall
    }
    fn exit_observation(&self) -> crate::outbound::execution::types::ExitObservationMode {
        crate::outbound::execution::types::ExitObservationMode::Direct
    }
    fn run(
        &self,
        _: Invocation,
    ) -> Result<ProcessOutput, ryo_benchmark_contracts::domain::reports::errors::ReportsError> {
        Ok(ProcessOutput {
            status: ProcessExitCode(2),
            signal: TerminationSignal::Exited,
            stdout: CapturedBytes(self.stdout.clone()),
            stderr: CapturedBytes(self.stderr.clone()),
            wall: std::time::Duration::ZERO,
            resources: ResourceObservation::WallOnly,
        })
    }
}

/// Explicit no-deadline capability; the underlying fixture retains actual invocation history.
pub(crate) struct Observer(pub crate::outbound::execution::fixtures::fake::ExecutionFake);
impl ExecutionPort for Observer {
    fn profile(&self) -> ExecutionProfile {
        self.0.profile()
    }
    fn exit_observation(&self) -> crate::outbound::execution::types::ExitObservationMode {
        crate::outbound::execution::types::ExitObservationMode::Direct
    }
    fn run(
        &self,
        invocation: Invocation,
    ) -> Result<ProcessOutput, ryo_benchmark_contracts::domain::reports::errors::ReportsError> {
        self.0.run(invocation)
    }
}

/// Decode the process-failure certificate a test authored as JSON into its contract value.
pub(crate) fn process_failure(
    value: &Value,
) -> ryo_benchmark_contracts::domain::process::types::ProcessFailure {
    use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
    use ryo_benchmark_contracts::domain::process::types::{
        OutputPart, OutputText, PathEncoding, ProcessArtifact, ProcessFailure, ProcessObservation,
    };
    let text = |value: &Value| value.as_str().expect("certificate text").to_owned();
    let parts = |value: &Value| -> Vec<OutputPart> {
        value
            .as_array()
            .expect("output parts")
            .iter()
            .map(|part| match part["kind"].as_str().expect("part kind") {
                "literal" => OutputPart::Literal(OutputText(text(&part["value"]))),
                _ => OutputPart::InputPath {
                    file: text(&part["file"]).into(),
                    encoding: match part["encoding"].as_str().expect("encoding") {
                        "utf8-bytes" => PathEncoding::Utf8Bytes,
                        _ => PathEncoding::JsonStringContent,
                    },
                },
            })
            .collect()
    };
    let artifact = |value: &Value| ProcessArtifact {
        path: text(&value["path"]).into(),
        sha256: ArtifactDigest(text(&value["sha256"])),
    };
    ProcessFailure {
        exit: ProcessExitCode(i32::try_from(value["exit"].as_i64().expect("exit")).expect("exit")),
        stdout: parts(&value["stdout"]),
        stderr: parts(&value["stderr"]),
        observations: value["observations"]
            .as_array()
            .expect("observations")
            .iter()
            .map(|row| ProcessObservation {
                executable_sha256: ArtifactDigest(text(&row["executable-sha256"])),
                invocation: artifact(&row["invocation"]),
                stdout: artifact(&row["stdout"]),
                stderr: artifact(&row["stderr"]),
            })
            .collect(),
    }
}
