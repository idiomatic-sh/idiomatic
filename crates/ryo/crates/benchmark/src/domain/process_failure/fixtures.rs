//! A process failure with two archived captures of one exact executable.
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::measurements::types::ProcessExitCode;
use ryo_benchmark_contracts::domain::process::types::{
    OutputPart, OutputText, PathEncoding, ProcessArtifact, ProcessFailure, ProcessObservation,
};

fn capture(attempt: usize, executable: &str) -> ProcessObservation {
    let artifact = |file: &str| ProcessArtifact {
        path: format!("archive-{attempt}/{file}").into(),
        sha256: ArtifactDigest(format!("{attempt}").repeat(64)),
    };
    ProcessObservation {
        executable_sha256: ArtifactDigest(executable.repeat(64)),
        invocation: artifact("invocation.json"),
        stdout: artifact("stdout.bin"),
        stderr: artifact("stderr.bin"),
    }
}

/// A completed numeric failure with a literal stdout and a declared input path in stderr.
pub(super) fn process() -> ProcessFailure {
    ProcessFailure {
        exit: ProcessExitCode(2),
        stdout: vec![OutputPart::Literal(OutputText("payload".into()))],
        stderr: vec![
            OutputPart::Literal(OutputText("open ".into())),
            OutputPart::InputPath {
                file: "case.py".into(),
                encoding: PathEncoding::Utf8Bytes,
            },
        ],
        observations: vec![capture(0, "a"), capture(1, "a")],
    }
}

/// The failure with its archived captures replaced.
pub(super) fn archived(observations: Vec<ProcessObservation>) -> ProcessFailure {
    ProcessFailure {
        observations,
        ..process()
    }
}

/// A capture of the executable named by one letter.
pub(super) fn of(attempt: usize, executable: &str) -> ProcessObservation {
    capture(attempt, executable)
}
