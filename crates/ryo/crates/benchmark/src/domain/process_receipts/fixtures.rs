//! A registered process-failure case and the reference observation, artifacts and invocation record that satisfy it.
use crate::domain::canonical_text::{service::text_digest, types::Document};
use crate::domain::process_failure::service::{group_digest, output_digest};
use crate::outbound::inventory::types::{
    ArgumentText, CompleteInvocation, InvocationDocument, NormalizedSettings, Pin, Signal,
    case_document::{CaseDocument, Comparison},
};
use crate::domain::inventory_receipts::fixtures::{observed_row, registered_case};
use crate::outbound::suite::types::{
    observation_document::{
        ExitStatus, ObservationDocument, ProcessArtifacts, ProcessObservation,
        ReferenceExpectation,
    },
    reference_discrepancy::{
        ArtifactPointer, DiscrepancyIdentity, DiscrepancyValidation, ExpectationKind,
        ObservedExecutable, ReferenceDiscrepancy,
    },
    text::{CanonicalJson, CaseName, PathText, ReasonText, RuleName, Sha256Text, ContestantSide, YesNo},
};
use crate::outbound::suite::types::process::{
    OutputPartWire, ProcessArtifactWire, ProcessFailureWire, ProcessObservationWire,
};
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::process::types::ProcessFailure;
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
    pub artifacts: BTreeMap<PathBuf, Vec<u8>>,
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

fn archived(attempt: usize) -> ProcessObservationWire {
    let artifact = |file: &str, letter: char| ProcessArtifactWire {
        path: format!("archive-{attempt}/{file}").into(),
        sha256: repeated(letter),
    };
    let letters = ['1', '2', '3', '4', '5', '6'];
    ProcessObservationWire {
        executable_sha256: repeated('a'),
        invocation: artifact("invocation.json", letters[attempt * 3]),
        stdout: artifact("stdout.bin", letters[attempt * 3 + 1]),
        stderr: artifact("stderr.bin", letters[attempt * 3 + 2]),
    }
}

/// The registered completed failure: literal streams and two archived captures of one binary.
pub(crate) fn wire() -> ProcessFailureWire {
    ProcessFailureWire::ProcessFailure {
        exit: 2,
        stdout: vec![OutputPartWire::Literal {
            value: "payload".into(),
        }],
        stderr: vec![OutputPartWire::Literal {
            value: "issue".into(),
        }],
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
    let process = ProcessFailure::try_from(wire()).expect("registered process failure");
    let key = crate::domain::process_output::service::observation_key(
        &crate::domain::process_output::types::ObservationIdentity {
            rule: crate::domain::canonical_text::types::Label::from("A001"),
            name: crate::domain::canonical_text::types::Label::from("matched"),
            side: crate::domain::canonical_text::types::Label::from("reference"),
        },
    )
    .0;
    let location = |file: &str| format!("attempt/{key}/{file}");
    let (stdout, stderr, record) = (b"payload".to_vec(), b"issue".to_vec(), b"{}".to_vec());
    let settings = Sha256Text(text_digest(&Document(CANONICAL.into())).0);
    let case = CaseDocument {
        rule: RuleName("A001".into()),
        name: CaseName("matched".into()),
        source_sha256: sha('e'),
        files: vec![path("case.py")],
        comparison: Comparison::Full,
        reference_discrepancy: Some(ReferenceDiscrepancy {
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
                file_metadata: None,
            },
            approved_executable_sha256: Vec::new(),
            reference_expected: ReferenceExpectation::Underivable,
            intended_expected: Vec::new(),
            reason: ReasonText("independent semantic authority".into()),
            process: Some(wire()),
        }),
        ..registered_case("A001", "matched")
    };
    let row = ObservationDocument {
        diagnostics: None,
        skipped: None,
        execution_metadata: None,
        coverage: None,
        actual_request_root: Some(path(ROOT)),
        raw_parity_matches_intended: Some(YesNo::No),
        discrepancy_validation: Some(DiscrepancyValidation {
            discrepancy_basis: DiscrepancyBasis::LanguageDefect,
            actual_executable_sha256: sha('a'),
            process_observation_group_sha256: Some(Sha256Text(
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
            file_metadata_sha256: None,
        }),
        process_observation: Some(ProcessObservation {
            kind: ExpectationKind::ProcessFailure,
            exit: ExitStatus(2),
            signal: None,
            actual_request_root: path(ROOT),
            observer_no_deadline: YesNo::Yes,
            executable_sha256: Some(sha('a')),
            settings_sha256: Some(settings.clone()),
            metadata_error: None,
            template_sha256: Some(Sha256Text(output_digest(&process).0)),
            artifacts: Some(ProcessArtifacts {
                stdout: pointer(&location("stdout.bin"), &stdout),
                stderr: pointer(&location("stderr.bin"), &stderr),
                invocation: pointer(&location("invocation.json"), &record),
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
pub(crate) fn process(documents: &mut Documents) -> &mut ProcessObservation {
    documents
        .row
        .process_observation
        .as_mut()
        .expect("the base row observes a process")
}

/// The stdout path of the base documents.
pub(crate) fn stdout_path(documents: &Documents) -> PathBuf {
    PathBuf::from(
        &documents
            .row
            .process_observation
            .as_ref()
            .and_then(|observed| observed.artifacts.as_ref())
            .expect("the base row points at artifacts")
            .stdout
            .path
            .0,
    )
}
