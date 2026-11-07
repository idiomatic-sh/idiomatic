//! The local receipts adapter runs the port's contract, then appends rows the inventory reads back.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::inventory::interface::InventoryPort as _;
use crate::outbound::receipts::errors::ReceiptsError;
use crate::outbound::receipts::factory::local;
use crate::outbound::receipts::fixtures::contract::check;
use crate::outbound::receipts::fixtures::samples::observation;
use crate::outbound::receipts::interface::ReceiptsPort as _;
use crate::outbound::receipts::types::{CapturedProcess, CaseReceipts, ReceiptPath};
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::documents::types::invocation_document::value::{FailedInvocation, InvocationDocument};
use ryo_benchmark_contracts::domain::documents::types::text::value::{Pin, Signal};
use ryo_benchmark_contracts::domain::documents::types::text::value::ExitStatus;
use ryo_benchmark_contracts::domain::documents::types::observation_document::value::{ObservationDocument, ProcessRecord};
use ryo_benchmark_contracts::domain::documents::types::reference_discrepancy::value::ExpectationKind;
use ryo_benchmark_contracts::domain::documents::types::text::value::{PathText, ReasonText, YesNo};
use ryo_benchmark_contracts::domain::measurements::types::CapturedBytes;
use std::collections::BTreeMap;
use std::path::Path;

fn inventory() -> impl crate::outbound::inventory::interface::InventoryPort {
    crate::outbound::inventory::factory::local()
}
fn physical_contract() {
    check(&local(CaseReceipts::Unrecorded), &CaseReceipts::Unrecorded);
}
/// A capture that could not record its process identity.
fn failed_invocation() -> InvocationDocument {
    InvocationDocument::Failed(FailedInvocation {
        actual_request_root: PathText("/cases/root".into()),
        files: Vec::new(),
        arguments: Vec::new(),
        environment: BTreeMap::new(),
        directory: PathText("/cases/root".into()),
        exit: ExitStatus(2),
        signal: Signal::Absent,
        observer_no_deadline: YesNo::Yes,
        metadata_error: ReasonText("failed".into()),
        source_sha256: Pin::Absent,
        file_metadata_sha256: Pin::Absent,
    })
}
fn row() -> ObservationDocument {
    let mut document = observation();
    document.process_observation = ryo_benchmark_contracts::domain::documents::types::observation_document::value::ReferenceProcess::Observed(ProcessRecord {
        kind: ExpectationKind::ProcessFailure,
        exit: ExitStatus(2),
        signal: Signal::Absent,
        actual_request_root: PathText("/cases/root".into()),
        observer_no_deadline: YesNo::Yes,
        executable_sha256: Pin::Absent,
        settings_sha256: Pin::Absent,
        metadata_error: ryo_benchmark_contracts::domain::documents::types::observation_document::value::RecordedReason::Absent,
        template_sha256: Pin::Absent,
        artifacts: ryo_benchmark_contracts::domain::documents::types::observation_document::value::ProcessArtifactsRecord::Unattached,
    });
    document
}
/// The raw artifacts attached to the process record of a row.
fn attached(document: &ObservationDocument) -> &ryo_benchmark_contracts::domain::documents::types::observation_document::value::ProcessArtifacts {
    match &document.process_observation {
        ryo_benchmark_contracts::domain::documents::types::observation_document::value::ReferenceProcess::Observed(process) => match &process.artifacts {
            ryo_benchmark_contracts::domain::documents::types::observation_document::value::ProcessArtifactsRecord::Attached(artifacts) => artifacts,
            ryo_benchmark_contracts::domain::documents::types::observation_document::value::ProcessArtifactsRecord::Unattached => panic!("attached artifacts"),
        },
        ryo_benchmark_contracts::domain::documents::types::observation_document::value::ReferenceProcess::NotObserved => panic!("observed process"),
    }
}
fn write(receipt: &Path, stdout: &[u8]) {
    local(CaseReceipts::Unrecorded)
        .record_case(
            ReceiptPath(receipt),
            row(),
            Some(CapturedProcess {
                key: &ArtifactDigest("a".repeat(64)),
                invocation: &failed_invocation(),
                stdout: &CapturedBytes(stdout.to_vec()),
                stderr: &CapturedBytes(b"error\n".to_vec()),
            }),
        )
        .unwrap();
}
fn raw_bytes_attempts_and_unreferenced_resume_history_are_retained() {
    let root = tempfile::tempdir().unwrap();
    let receipt = root.path().join("001.cases.jsonl");
    write(&receipt, &[255, 0, 1]);
    let first = inventory()
        .observations(&receipt)
        .unwrap();
    let pointer = attached(&first.records[0])
        .stdout
        .path
        .0
        .as_str();
    assert_eq!(
        first.artifacts.get(Path::new(pointer)),
        Some(&vec![255, 0, 1])
    );
    std::fs::create_dir(
        root.path()
            .join("001.cases.jsonl.process-orphan"),
    )
    .unwrap();
    write(&receipt, b"second");
    let next = inventory()
        .observations(&receipt)
        .unwrap();
    assert_eq!(next.records.len(), 2);
    assert_eq!(next.artifacts.len(), 6);
    assert_eq!(
        next.artifacts.get(Path::new(pointer)),
        Some(&vec![255, 0, 1])
    );
    assert!(
        root.path()
            .join("001.cases.jsonl.process-orphan")
            .is_dir()
    );
}
fn a_sibling_receipt_cannot_reuse_another_shards_artifact_namespace() {
    let root = tempfile::tempdir().unwrap();
    let receipt = root.path().join("001.cases.jsonl");
    write(&receipt, b"first");
    let sibling = root.path().join("002.cases.jsonl");
    std::fs::copy(&receipt, &sibling).unwrap();
    assert!(matches!(
        inventory().observations(&sibling),
        Err(_refused)
    ));
    assert!(matches!(
        inventory().observations(&receipt),
        Ok(_loaded)
    ));
}
fn changed_raw_bytes_and_extra_files_in_referenced_attempts_are_refused() {
    let root = tempfile::tempdir().unwrap();
    let receipt = root.path().join("cases.jsonl");
    write(&receipt, b"first");
    let loaded = inventory()
        .observations(&receipt)
        .unwrap();
    let pointer = attached(&loaded.records[0])
        .stdout
        .path
        .0
        .as_str();
    let path = root.path().join(pointer);
    std::fs::write(&path, b"changed").unwrap();
    assert!(matches!(
        inventory().observations(&receipt),
        Err(_refused)
    ));
    std::fs::write(&path, b"first").unwrap();
    std::fs::write(
        path.with_file_name("unexpected.bin"),
        b"extra",
    )
    .unwrap();
    assert!(matches!(
        inventory().observations(&receipt),
        Err(_refused)
    ));
}
/// Replace the artifact at `path` with a link to `target`.
#[cfg(unix)]
fn link(target: &Path, path: &Path) {
    std::os::unix::fs::symlink(target, path).unwrap();
}
/// Platforms without symlinks leave the artifact where it is.
#[cfg(not(unix))]
fn link(_target: &Path, _path: &Path) {}
fn a_raw_artifact_symlink_cannot_escape_the_receipt_boundary() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let receipt = root.path().join("cases.jsonl");
    write(&receipt, b"first");
    let loaded = inventory()
        .observations(&receipt)
        .unwrap();
    let pointer = attached(&loaded.records[0])
        .stdout
        .path
        .0
        .as_str();
    let path = root.path().join(pointer);
    let target = outside.path().join("bytes");
    std::fs::write(&target, b"first").unwrap();
    std::fs::remove_file(&path).unwrap();
    link(&target, &path);
    assert!(matches!(
        inventory().observations(&receipt),
        Err(_refused)
    ));
}

fn an_unsafe_receipt_name_is_an_invalid_receipt() {
    let root = tempfile::tempdir().unwrap();
    let refused = local(CaseReceipts::Unrecorded).record_case(
        ReceiptPath(&root.path().join("bad:name.jsonl")),
        row(),
        None,
    );
    assert!(matches!(refused, Err(ReceiptsError::Invalid(_))));
}

type Scenario = fn();
type Input = Scenario;
type Value = ();
type Failure = std::convert::Infallible;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("physical_contract"),
        word: Word::Succeeds,
        input: || physical_contract as fn(),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("raw_bytes_attempts_and_unreferenced_resume_history_are_retained"),
        word: Word::Succeeds,
        input: || raw_bytes_attempts_and_unreferenced_resume_history_are_retained as fn(),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_sibling_receipt_cannot_reuse_another_shards_artifact_namespace"),
        word: Word::Succeeds,
        input: || a_sibling_receipt_cannot_reuse_another_shards_artifact_namespace as fn(),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("changed_raw_bytes_and_extra_files_in_referenced_attempts_are_refused"),
        word: Word::Succeeds,
        input: || changed_raw_bytes_and_extra_files_in_referenced_attempts_are_refused as fn(),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("an_unsafe_receipt_name_is_an_invalid_receipt"),
        word: Word::Succeeds,
        input: || an_unsafe_receipt_name_is_an_invalid_receipt as fn(),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_raw_artifact_symlink_cannot_escape_the_receipt_boundary"),
        word: Word::Succeeds,
        input: || a_raw_artifact_symlink_cannot_escape_the_receipt_boundary as fn(),
        expect: Expect::Value(()),
    },
];
fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario();
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
