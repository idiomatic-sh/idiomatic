//! A case declaring a completed process failure, the proof that validated it and the capture that satisfies both.
use crate::domain::discrepancy::fixtures::case;
use crate::domain::discrepancy::types::{OptionalDigest, ReferenceKind, ReferenceProof};
use ryo_benchmark_contracts::domain::documents::types::invocation_document::value::{ArgumentText, CompleteInvocation, InvocationDocument, NormalizedSettings};
use ryo_benchmark_contracts::domain::documents::types::text::value::{Pin, Signal};
use ryo_benchmark_contracts::domain::documents::types::text::value::{CanonicalJson, ExitStatus, PathText, ReasonText, Sha256Text, YesNo};
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::measurements::types::{
    CapturedBytes, ProcessExitCode, ProcessOutput, ResourceObservation,
};
use ryo_benchmark_contracts::domain::process::types::{
    OutputPart, ProcessFailure, ReferenceExpectation,
};
use ryo_benchmark_contracts::domain::workloads::types::{DiscrepancyBasis, PreparedCase};
use std::collections::BTreeMap;

/// Whether the independent discrepancy artifacts verify.
#[derive(Clone, Copy, Debug)]
pub(super) enum Artifacts {
    /// Every artifact verifies.
    Valid,
    /// An artifact is stale.
    Invalid,
}

/// The case, what was captured of its reference and the proof that validated it.
pub(super) struct Actual {
    pub case: PreparedCase,
    pub invocation: InvocationDocument,
    pub output: ProcessOutput,
    pub proof: ReferenceProof,
    pub rules: &'static str,
    pub artifacts: Artifacts,
}

fn repeated(letter: char) -> String {
    letter.to_string().repeat(64)
}

fn digest(letter: char) -> ArtifactDigest {
    ArtifactDigest(repeated(letter))
}

fn path(text: &str) -> PathText {
    PathText(text.into())
}

/// The captured invocation record of a completed failure.
pub(super) fn record() -> CompleteInvocation {
    CompleteInvocation {
        actual_request_root: path("/root"),
        files: vec![path("case.py")],
        executable_sha256: Sha256Text(repeated('a')),
        settings_sha256: Sha256Text(repeated('d')),
        normalized_settings: NormalizedSettings {
            arguments: vec![ArgumentText("--check".into())],
            environment: BTreeMap::new(),
            directory: path("$ROOT"),
            canonical: CanonicalJson("{}".into()),
        },
        profile_sha256: Sha256Text(repeated('c')),
        tool: ReasonText("fixture".into()),
        version: ReasonText("1".into()),
        state: path("/state"),
        arguments: vec![ArgumentText("--check".into())],
        environment: BTreeMap::new(),
        directory: path("/root"),
        exit: ExitStatus(2),
        signal: Signal::Absent,
        observer_no_deadline: YesNo::Yes,
        source_sha256: Pin::Absent,
        file_metadata_sha256: Pin::Absent,
    }
}

/// A process failure whose bytes are both literal.
pub(super) fn passing() -> Actual {
    let mut prepared = case();
    prepared.request.root = "/root".into();
    let ryo_benchmark_contracts::domain::workloads::types::PreparedReferenceBehavior::Differs(discrepancy) = &mut prepared.reference_discrepancy else {
        panic!("fixture discrepancy");
    };
    discrepancy.expected = ReferenceExpectation::ProcessFailure(ProcessFailure {
        exit: ryo_benchmark_contracts::domain::measurements::types::ProcessExitCode(2),
        stdout: vec![OutputPart::Literal(ryo_benchmark_contracts::domain::process::types::OutputText("payload".into()))],
        stderr: vec![OutputPart::Literal(ryo_benchmark_contracts::domain::process::types::OutputText("issue".into()))],
        observations: Vec::new(),
    });
    Actual {
        case: prepared,
        invocation: InvocationDocument::Complete(record()),
        output: ProcessOutput {
            signal: ryo_benchmark_contracts::domain::measurements::types::TerminationSignal::Exited,
            status: ProcessExitCode(2),
            stdout: CapturedBytes(b"payload".to_vec()),
            stderr: CapturedBytes(b"issue".to_vec()),
            wall: std::time::Duration::ZERO,
            resources: ResourceObservation::WallOnly,
        },
        proof: ReferenceProof {
            basis: DiscrepancyBasis::LanguageDefect,
            process_group: OptionalDigest::Absent,
            kind: ReferenceKind::ProcessFailure,
            settings: digest('d'),
            executable: digest('a'),
            profile: digest('c'),
            source: digest('e'),
            reference_output: digest('f'),
            intended_output: digest('0'),
            file_metadata: OptionalDigest::Absent,
        },
        rules: "rules/benchmark.toml",
        artifacts: Artifacts::Valid,
    }
}

/// The passing capture with one edit.
pub(super) fn edited(edit: impl FnOnce(&mut Actual)) -> Actual {
    let mut actual = passing();
    edit(&mut actual);
    actual
}

/// The captured record of the passing capture, edited in place.
pub(super) fn complete(actual: &mut Actual) -> &mut CompleteInvocation {
    match &mut actual.invocation {
        InvocationDocument::Complete(record) => record,
        InvocationDocument::Failed(_) => unreachable!("the passing capture is complete"),
    }
}
