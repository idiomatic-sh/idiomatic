//! A registered process-failure case and the reference observation, artifacts and invocation record that satisfy it.
use crate::domain::canonical_text::{service::text_digest, types::Document};
use crate::domain::inventory_receipts::fixtures::{observed_row, registered_case};
use crate::domain::process_failure::service::{group_digest, output_digest};
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::documents::types::case_document::value::CaseDocument;
use ryo_benchmark_contracts::domain::documents::types::case_document::value::CaseReference;
use ryo_benchmark_contracts::domain::documents::types::finding::value::ExpectedFindings;
use ryo_benchmark_contracts::domain::documents::types::invocation_document::value::{
    ArgumentText, CompleteInvocation, InvocationDocument, NormalizedSettings,
};
use ryo_benchmark_contracts::domain::documents::types::observation_document::value::{
    IntentMatch, ObservationDocument, ProcessArtifacts, ProcessArtifactsRecord, ProcessRecord,
    RecordedMetadata, RecordedReason, ReferenceProcess, ReportedFindings, ReportedSkips,
    RequestRoot, ValidationRecord,
};
use ryo_benchmark_contracts::domain::documents::types::reference_discrepancy::value::{
    ArtifactPointer, DiscrepancyIdentity, DiscrepancyRecord, DiscrepancyValidation,
    ExpectationKind, ObservedExecutable, ProcessExpectation,
};
use ryo_benchmark_contracts::domain::documents::types::text::value::{
    CanonicalJson, CaseName, ContestantSide, ExitStatus, FileMetadataDigest, PathText, ReasonText,
    RuleName, Sha256Text, YesNo,
};
use ryo_benchmark_contracts::domain::documents::types::text::value::{Pin, Signal};
use ryo_benchmark_contracts::domain::measurements::types::CapturedBytes;
use ryo_benchmark_contracts::domain::measurements::types::ProcessExitCode;
use ryo_benchmark_contracts::domain::process::types::{
    OutputPart, OutputText, ProcessArtifact, ProcessFailure, ProcessObservation as ProcessCapture,
};
use ryo_benchmark_contracts::domain::reports::types::CoverageAccounting;
use ryo_benchmark_contracts::domain::workloads::types::CaseComparison;
use ryo_benchmark_contracts::domain::workloads::types::DiscrepancyBasis;
use sha2::Digest as _;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// The documents one reference process receipt is validated over.
#[derive(Clone)]
pub(crate) struct Documents {
    /// The registered case.
    pub case: CaseDocument,
    /// The reference side's observation row.
    pub row: ObservationDocument,
    /// The raw bytes the row points at.
    pub artifacts: BTreeMap<PathBuf, CapturedBytes>,
    /// The raw invocation record the row points at.
    pub invocations: BTreeMap<PathBuf, InvocationDocument>,
}

const ROOT: &str = "/root";
const STATE: &str = "/state";
const CANONICAL: &str =
    "{\"arguments\":[\"--check\",\"$ROOT/case.py\"],\"directory\":\"$ROOT\",\"environment\":{}}";

fn repeated(letter: char) -> String {
    letter.to_string().repeat(64)
}

fn sha(letter: char) -> Sha256Text {
    Sha256Text(repeated(letter))
}

fn path(text: &str) -> PathText {
    PathText(text.into())
}

fn bytes_digest(bytes: &[u8]) -> Sha256Text {
    Sha256Text(format!("{:x}", sha2::Sha256::digest(bytes)))
}

fn archived(attempt: usize) -> ProcessCapture {
    let artifact = |file: &str, letter: char| ProcessArtifact {
        path: format!("archive-{attempt}/{file}").into(),
        sha256: ArtifactDigest(repeated(letter)),
    };
    let letters = ['1', '2', '3', '4', '5', '6'];
    ProcessCapture {
        executable_sha256: ArtifactDigest(repeated('a')),
        invocation: artifact("invocation.json", letters[attempt * 3]),
        stdout: artifact("stdout.bin", letters[attempt * 3 + 1]),
        stderr: artifact("stderr.bin", letters[attempt * 3 + 2]),
    }
}

/// The registered completed failure: literal streams and two archived captures of one binary.
pub(crate) fn failure() -> ProcessFailure {
    ProcessFailure {
        exit: ProcessExitCode(2),
        stdout: vec![OutputPart::Literal(OutputText("payload".into()))],
        stderr: vec![OutputPart::Literal(OutputText("issue".into()))],
        observations: vec![archived(0), archived(1)],
    }
}

fn pointer(location: &str, bytes: &[u8]) -> ArtifactPointer {
    ArtifactPointer {
        path: path(location),
        sha256: bytes_digest(bytes),
    }
}

/// The documents of a reference process receipt that validates.
pub(crate) fn passing() -> Documents {
    let process = failure();
    let key = crate::domain::process_output::service::observation_key(
        &crate::domain::process_output::types::ReceiptKeyParts {
            rule: crate::domain::canonical_text::types::Label::from("A001"),
            name: crate::domain::canonical_text::types::Label::from("matched"),
            side: crate::domain::canonical_text::types::Label::from("reference"),
        },
    )
    .0;
    let location = |file: &str| format!("attempt/{key}/{file}");
    let (stdout, stderr, record) = (
        CapturedBytes(b"payload".to_vec()),
        CapturedBytes(b"issue".to_vec()),
        CapturedBytes(b"{}".to_vec()),
    );
    let settings = Sha256Text(text_digest(&Document(CANONICAL.into())).0);
    let case = CaseDocument {
        rule: RuleName("A001".into()),
        name: CaseName("matched".into()),
        source_sha256: sha('e'),
        files: vec![path("case.py")],
        comparison: CaseComparison::Full,
        reference_discrepancy: CaseReference::Differs(DiscrepancyRecord {
            basis: DiscrepancyBasis::LanguageDefect,
            identity: DiscrepancyIdentity {
                tool: ReasonText("fixture".into()),
                version: ReasonText("1".into()),
                observed_executable_sha256: ObservedExecutable::NotObserved,
                profile: sha('c'),
                settings: settings.clone(),
                source: sha('e'),
                reference_output: sha('d'),
                intended_output: sha('f'),
                file_metadata: FileMetadataDigest::Undeclared,
            },
            approved_executable_sha256: Vec::new(),
            reference_expected: ExpectedFindings::Underivable,
            intended_expected: Vec::new(),
            reason: ReasonText("independent semantic authority".into()),
            process: ProcessExpectation::Failure(failure()),
        }),
        ..registered_case("A001", "matched")
    };
    let row = ObservationDocument {
        diagnostics: ReportedFindings::Unreported,
        skipped: ReportedSkips::Unreported,
        execution_metadata: RecordedMetadata::Unrecorded,
        coverage: CoverageAccounting::Unaccounted,
        actual_request_root: RequestRoot::Written(path(ROOT)),
        raw_parity_matches_intended: IntentMatch::Matched(YesNo::No),
        discrepancy_validation: ValidationRecord::Attested(DiscrepancyValidation {
            discrepancy_basis: DiscrepancyBasis::LanguageDefect,
            actual_executable_sha256: sha('a'),
            process_observation_group_sha256: Pin::Sha256(Sha256Text(
                group_digest(&process, &ArtifactDigest(repeated('a')))
                    .expect("group")
                    .0,
            )),
            reference_expectation_kind: ExpectationKind::ProcessFailure,
            settings_sha256: settings.clone(),
            profile_sha256: sha('c'),
            source_sha256: sha('e'),
            reference_output_sha256: sha('d'),
            intended_output_sha256: sha('f'),
            file_metadata_sha256: FileMetadataDigest::Undeclared,
        }),
        process_observation: ReferenceProcess::Observed(ProcessRecord {
            kind: ExpectationKind::ProcessFailure,
            exit: ExitStatus(2),
            signal: Signal::Absent,
            actual_request_root: path(ROOT),
            observer_no_deadline: YesNo::Yes,
            executable_sha256: Pin::Sha256(sha('a')),
            settings_sha256: Pin::Sha256(settings.clone()),
            metadata_error: RecordedReason::Absent,
            template_sha256: Pin::Sha256(Sha256Text(output_digest(&process).0)),
            artifacts: ProcessArtifactsRecord::Attached(ProcessArtifacts {
                stdout: pointer(&location("stdout.bin"), &stdout.0),
                stderr: pointer(&location("stderr.bin"), &stderr.0),
                invocation: pointer(&location("invocation.json"), &record.0),
            }),
        }),
        ..observed_row("A001", "matched", ContestantSide::Reference)
    };
    let invocation = CompleteInvocation {
        actual_request_root: path(ROOT),
        files: vec![path("case.py")],
        executable_sha256: sha('a'),
        settings_sha256: settings,
        normalized_settings: NormalizedSettings {
            arguments: vec![
                ArgumentText("--check".into()),
                ArgumentText("$ROOT/case.py".into()),
            ],
            environment: BTreeMap::new(),
            directory: path("$ROOT"),
            canonical: CanonicalJson(CANONICAL.into()),
        },
        profile_sha256: sha('c'),
        tool: ReasonText("fixture".into()),
        version: ReasonText("1".into()),
        state: path(STATE),
        arguments: vec![
            ArgumentText("--check".into()),
            ArgumentText(format!("{ROOT}/case.py")),
        ],
        environment: BTreeMap::new(),
        directory: path(ROOT),
        exit: ExitStatus(2),
        signal: Signal::Absent,
        observer_no_deadline: YesNo::Yes,
        source_sha256: Pin::Sha256(sha('e')),
        file_metadata_sha256: Pin::Absent,
    };
    Documents {
        case,
        row,
        artifacts: BTreeMap::from([
            (PathBuf::from(location("stdout.bin")), stdout),
            (PathBuf::from(location("stderr.bin")), stderr),
            (PathBuf::from(location("invocation.json")), record),
        ]),
        invocations: BTreeMap::from([(
            PathBuf::from(location("invocation.json")),
            InvocationDocument::Complete(invocation),
        )]),
    }
}

/// A base set of documents with one edit.
pub(crate) fn edited(base: fn() -> Documents, edit: impl FnOnce(&mut Documents)) -> Documents {
    let mut documents = base();
    edit(&mut documents);
    documents
}

/// The raw invocation record of the base documents, edited in place.
pub(crate) fn invocation(documents: &mut Documents) -> &mut CompleteInvocation {
    match documents.invocations.values_mut().next() {
        Some(InvocationDocument::Complete(record)) => record,
        _ => unreachable!("the base documents hold one complete record"),
    }
}

/// The observed process of the base documents, edited in place.
pub(crate) fn process(documents: &mut Documents) -> &mut ProcessRecord {
    match &mut documents.row.process_observation {
        ReferenceProcess::Observed(record) => record,
        ReferenceProcess::NotObserved => panic!("the base row observes a process"),
    }
}

/// The stdout path of the base documents.
pub(crate) fn stdout_path(documents: &Documents) -> PathBuf {
    let ReferenceProcess::Observed(observed) = &documents.row.process_observation else {
        panic!("the base row observes a process");
    };
    let ProcessArtifactsRecord::Attached(artifacts) = &observed.artifacts else {
        panic!("the base row points at artifacts");
    };
    PathBuf::from(&artifacts.stdout.path.0)
}

/// The registered discrepancy of the base documents, edited in place.
pub(crate) fn discrepancy(documents: &mut Documents) -> &mut DiscrepancyRecord {
    match &mut documents.case.reference_discrepancy {
        CaseReference::Differs(record) => record,
        CaseReference::Agrees => panic!("the base case registers a discrepancy"),
    }
}

/// The attested validation of the base row, edited in place.
pub(crate) fn validation(documents: &mut Documents) -> &mut DiscrepancyValidation {
    match &mut documents.row.discrepancy_validation {
        ValidationRecord::Attested(proof) => proof,
        ValidationRecord::Unattested => panic!("the base row attests a validation"),
    }
}

/// The raw artifacts of the observed process of the base row, edited in place.
pub(crate) fn artifacts(documents: &mut Documents) -> &mut ProcessArtifacts {
    match &mut process(documents).artifacts {
        ProcessArtifactsRecord::Attached(artifacts) => artifacts,
        ProcessArtifactsRecord::Unattached => panic!("the base row points at artifacts"),
    }
}
