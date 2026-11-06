//! A prepared case with a pinned reference, the suite that serves its profile and digests, and the pinned reference linter.
use super::service::settings_text;
use crate::domain::canonical_text::service::text_digest;
use crate::domain::discrepancy::fixtures::{case, evidence, profile};
use crate::domain::discrepancy::service::{output_digest, reference_digest};
use crate::outbound::linter::{fixtures::fake::LinterFake, interface::LinterPort as _};
use crate::outbound::suite::fixtures::fake::SuiteFake;
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::process::types::{
    OutputPart, ProcessArtifact, ProcessFailure, ProcessObservation,
};
use ryo_benchmark_contracts::domain::workloads::types::PreparedCase;
use sha2::Digest as _;
use std::path::PathBuf;

/// Whether the suite serves the profile.
#[derive(Clone, Copy, Debug)]
pub(super) enum Served {
    /// The profile and its digests are readable.
    Profile,
    /// The suite holds nothing.
    Nothing,
}

/// How the archived invocation record of every capture reads.
#[derive(Clone, Copy, Debug)]
pub(super) enum Record {
    /// The capture could not record its process identity.
    Failed,
    /// The identity matches the certificate but the settings are not those the arguments establish.
    Unestablished,
}

/// The archived captures of a process failure, written under one profile root.
pub(super) struct Archive {
    pub root: tempfile::TempDir,
    pub process: ProcessFailure,
}

/// What a row validates.
pub(super) enum Step {
    /// The whole certificate of the case.
    Validate,
    /// Only the archived captures of its process failure.
    Archives(Archive),
}

/// The case, the rules path and the executable the suite digests.
pub(super) struct Validation {
    pub case: PreparedCase,
    pub rules: &'static str,
    pub executable: ArtifactDigest,
    pub served: Served,
    pub step: Step,
}

/// The suite that serves the fixture profile and the digests of the executable and the profile file.
pub(super) fn suite(validation: &Validation) -> SuiteFake {
    let mut suite = SuiteFake {
        profile: match validation.served {
            Served::Profile => Some(profile()),
            Served::Nothing => None,
        },
        discrepancy_artifacts_valid: true,
        ..SuiteFake::default()
    };
    suite.artifact_digests.extend([
        (PathBuf::from("fixture"), validation.executable.clone()),
        (
            PathBuf::from("rules/benchmark.toml"),
            ArtifactDigest("c".repeat(64)),
        ),
    ]);
    suite
}

/// The case as registered, with the language evidence its profile supports.
pub(super) fn evidenced() -> Validation {
    let mut prepared = case();
    prepared
        .reference_discrepancy
        .as_mut()
        .expect("fixture discrepancy")
        .evidence
        .push(evidence());
    Validation {
        case: prepared,
        rules: "rules/benchmark.toml",
        executable: ArtifactDigest("a".repeat(64)),
        served: Served::Profile,
        step: Step::Validate,
    }
}

/// The case whose certificate binds the digests of this very case, profile and invocation.
pub(super) fn certified() -> Validation {
    let mut validation = evidenced();
    let invocation = LinterFake::default()
        .prepare(&validation.case.request)
        .expect("fixture invocation");
    let settings = text_digest(
        &settings_text(&invocation, &validation.case, &profile()).expect("fixture settings"),
    );
    let expected = output_digest(&validation.case.expected);
    let item = validation
        .case
        .reference_discrepancy
        .as_mut()
        .expect("fixture discrepancy");
    item.identity.settings_sha256 = settings;
    item.identity.intended_output_sha256 = expected;
    item.identity.reference_output_sha256 = reference_digest(&item.expected);
    validation
}

/// The case with its registered discrepancy edited.
pub(super) fn edited(base: fn() -> Validation, edit: impl FnOnce(&mut PreparedCase)) -> Validation {
    let mut validation = base();
    edit(&mut validation.case);
    validation
}

fn record_text(record: Record, root: &str) -> Vec<u8> {
    let failed = serde_json::json!({
        "actual_request_root": root, "files": ["case.py"], "arguments": [], "environment": {},
        "directory": ".", "exit": 2, "signal": null, "observer_no_deadline": true,
        "metadata_error": "identity unavailable",
    });
    let unestablished = serde_json::json!({
        "actual_request_root": root, "files": ["case.py"],
        "executable_sha256": "a".repeat(64), "settings_sha256": "d".repeat(64),
        "normalized_settings": {"arguments": [], "environment": {}, "directory": "."},
        "profile_sha256": "c".repeat(64), "tool": "fixture", "version": "1.0", "state": "cache",
        "arguments": [], "environment": {}, "directory": ".", "exit": 2, "signal": null,
        "observer_no_deadline": true, "source_sha256": "e".repeat(64),
    });
    match record {
        Record::Failed => failed.to_string().into_bytes(),
        Record::Unestablished => unestablished.to_string().into_bytes(),
    }
}

/// A process failure whose two archived captures of the approved binary are written under a profile root.
pub(super) fn archived(record: Record) -> Validation {
    let root = tempfile::tempdir().expect("archive root");
    let location = root.path().to_string_lossy().into_owned();
    let mut observations = Vec::new();
    for attempt in 0..2 {
        let directory = root.path().join(format!("archive-{attempt}"));
        std::fs::create_dir(&directory).expect("archive directory");
        let mut artifact = |file: &str, bytes: &[u8]| {
            std::fs::write(directory.join(file), bytes).expect("archived bytes");
            ProcessArtifact {
                path: PathBuf::from(format!("archive-{attempt}/{file}")),
                sha256: ArtifactDigest(format!("{:x}", sha2::Sha256::digest(bytes))),
            }
        };
        observations.push(ProcessObservation {
            executable_sha256: ArtifactDigest("a".repeat(64)),
            invocation: artifact("invocation.json", &record_text(record, &location)),
            stdout: artifact("stdout.bin", b"payload"),
            stderr: artifact("stderr.bin", b"issue"),
        });
    }
    Validation {
        step: Step::Archives(Archive {
            root,
            process: ProcessFailure {
                exit: 2,
                stdout: vec![OutputPart::Literal("payload".into())],
                stderr: vec![OutputPart::Literal("issue".into())],
                observations,
            },
        }),
        ..evidenced()
    }
}
