//! Real verification capture, boundary archive loading, and strict offline comparison.
use crate::application::case_execution::service::run as run_case;
use crate::application::case_observation::service::write as record;
use crate::application::process_discrepancy::errors::ProcessDiscrepancyError;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::discrepancy::types::Contestant;
use crate::domain::inventory_receipts::types::{ReceiptCounts, Tally};
use crate::domain::process_failure::types::{CaptureRecord, ProcessCapture};
use crate::inbound::use_cases::verify_cases::fixtures::process_capture::{Fixture, context, file_member};
use crate::outbound::execution::fixtures::{fake::ExecutionFake, observers::Observer};
use crate::outbound::inventory::fixtures::registrations::complete_of;
use crate::outbound::linter::fixtures::fake::LinterFake;
use crate::outbound::receipts::{fixtures::fake::ReceiptsFake, types::CaseReceipts};
use ryo_benchmark_contracts::domain::documents::types::invocation_document::value::InvocationDocument;
use ryo_benchmark_contracts::domain::documents::types::observation_document::value::{
    IntentMatch, RecordedReason, ReportedFindings,
};
use ryo_benchmark_contracts::domain::documents::types::text::value::{ReasonText, YesNo};
use ryo_benchmark_contracts::domain::measurements::types::{SignalNumber, TerminationSignal};
use ryo_benchmark_contracts::domain::receipts::types::CaseKey;
use ryo_benchmark_contracts::domain::requests::types::RuleLabel;
use ryo_benchmark_contracts::domain::workloads::types::benchmark_case_name::value::BenchmarkCaseName;

/// What a capture scenario establishes once its assertions hold.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Holds,
}

/// Which capture behavior a row exercises.
#[derive(Clone, Copy)]
enum Scenario {
    ProvidersArchiveAndValidate,
    MetadataFailureKeepsBytes,
    TerminationsAreNotExpectedFailures,
}

type Input = Scenario;
type Value = Outcome;
type Failure = ();

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("two tool neutral providers capture, archive and validate without a report"),
        word: Word::Succeeds,
        input: || Scenario::ProvidersArchiveAndValidate,
        expect: Expect::Value(Outcome::Holds),
    },
    Case {
        name: CaseName("a metadata failure preserves the process bytes and cannot be admitted"),
        word: Word::Succeeds,
        input: || Scenario::MetadataFailureKeepsBytes,
        expect: Expect::Value(Outcome::Holds),
    },
    Case {
        name: CaseName("decoded success, deadline observers, signals and extra backtraces are not expected failures"),
        word: Word::Succeeds,
        input: || Scenario::TerminationsAreNotExpectedFailures,
        expect: Expect::Value(Outcome::Holds),
    },
];

/// Calls the scenario on one input.
fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::ProvidersArchiveAndValidate => providers_archive_and_validate(),
        Scenario::MetadataFailureKeepsBytes => metadata_failure_keeps_bytes(),
        Scenario::TerminationsAreNotExpectedFailures => terminations_are_not_expected_failures(),
    }
    Ok(Outcome::Holds)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

/// The captured reference process matched against the fixture's declared failure and its proof.
fn matches_capture(fixture: &Fixture, capture: &ProcessCapture) -> Result<(), ProcessDiscrepancyError> {
    crate::application::process_discrepancy::service::matches(
        &fixture.prepared,
        &capture.invocation,
        &capture.output,
        &fixture.proof,
        &fixture.suite,
        &fixture.rules,
    )
}

fn providers_archive_and_validate() {
    for provider in ["provider-one", "provider-two"] {
        let fixture = if provider == "provider-one" {
            Fixture::new(provider)
        } else {
            Fixture::with_paths(provider)
        };
        let captured = fixture.load();
        assert_eq!(captured.records.len(), 2);
        assert_eq!(captured.artifacts.len(), 3);
        if provider == "provider-two" {
            let (stdout, stderr) = captured.reference_streams();
            assert_eq!(
                file_member(stdout),
                fixture
                    .prepared
                    .request
                    .root
                    .join("case.py")
                    .to_str()
                    .unwrap()
            );
            assert!(String::from_utf8_lossy(&stdout.0).contains("日本"));
            if cfg!(unix) {
                assert!(
                    String::from_utf8_lossy(&stdout.0).contains("\\\\"),
                    "JSON encoding must preserve and escape a real backslash in the root"
                );
                assert!(
                    String::from_utf8_lossy(&stdout.0).contains("\\\""),
                    "JSON encoding must escape a real quote in the root"
                );
            }
            assert_eq!(
                stderr.0,
                format!(
                    "open {}\n",
                    fixture.prepared.request.root.join("case.py").display()
                )
                .as_bytes()
            );
        }
        assert_eq!(captured.records[0].diagnostics, ReportedFindings::Unreported);
        assert_eq!(
            captured.records[0].raw_parity_matches_intended,
            IntentMatch::Matched(YesNo::No)
        );
        assert_eq!(
            crate::domain::inventory_receipts::service::validate_with_artifacts(
                &[fixture.document.clone()],
                &captured.records,
                &captured.artifacts,
                &captured.invocations
            ),
            Ok(ReceiptCounts {
                raw_exact: Tally(0),
                documented: Tally(1),
                contract_only: Tally(0)
            })
        );
        let audit = crate::application::receipt_validation::service::validate(
            &fixture.registration(),
            &captured.records,
            &captured.raw(),
        )
        .unwrap();
        assert!(audit.errors.is_empty());
        assert_eq!(
            audit.verified,
            vec![CaseKey {
                rule: RuleLabel("A001".into()),
                name: BenchmarkCaseName(provider.into()),
            }]
        );
        assert_eq!(
            (
                audit.raw_exact.0,
                audit.documented_discrepancies.0,
                audit.contract_only.0
            ),
            (0, 1, 0)
        );
    }
}

fn metadata_failure_keeps_bytes() {
    let fixture = Fixture::new("metadata");
    let mut suite = fixture.suite.clone();
    suite.artifact_digests.clear();
    let linter = LinterFake::default();
    let execution = Observer(ExecutionFake {
        status: 2,
        ..Default::default()
    });
    let context = context();
    let outcome = run_case(
        &suite,
        &context,
        &linter,
        &execution,
        &fixture.prepared,
        &fixture.rules,
    );
    assert_eq!(outcome.result.unwrap_err().to_string(), "operational failure");
    let CaptureRecord::Kept(kept) = &outcome.capture else {
        panic!("an undecodable output keeps its raw bytes");
    };
    assert_eq!(kept.output.stdout.0, b"payload");
    assert!(matches!(kept.invocation, InvocationDocument::Failed(_)));
    let receipt = fixture.root.path().join("failed.jsonl");
    let receipts = ReceiptsFake {
        case_receipts: CaseReceipts::Recorded(receipt.clone()),
    };
    record(
        &suite,
        &receipts,
        &context,
        &fixture.prepared,
        Contestant::Reference,
        None,
        Some(&ReasonText("metadata unavailable".into())),
        YesNo::No,
        None,
        &outcome.capture,
    )
    .unwrap();
    let loaded = fixture.load_file(&receipt);
    assert_eq!(loaded.artifacts.len(), 3);
    assert_eq!(loaded.records[0].case_passed, YesNo::No);
    assert!(matches!(loaded.records[0].error, RecordedReason::Stated(_)));
}

fn terminations_are_not_expected_failures() {
    let fixture = Fixture::new("termination");
    let linter = LinterFake::default();
    let context = context();
    for status in [0, 1] {
        let execution = Observer(ExecutionFake {
            status,
            ..Default::default()
        });
        let outcome = run_case(
            &fixture.suite,
            &context,
            &linter,
            &execution,
            &fixture.prepared,
            &fixture.rules,
        );
        outcome.result.unwrap();
        assert!(
            matches!(outcome.capture, CaptureRecord::Absent),
            "a decoded report cannot satisfy a process expectation"
        );
    }
    let bounded = ExecutionFake {
        status: 2,
        ..Default::default()
    };
    let CaptureRecord::Kept(mut capture) = run_case(
        &fixture.suite,
        &context,
        &linter,
        &bounded,
        &fixture.prepared,
        &fixture.rules,
    )
    .capture
    else {
        panic!("an undecodable output keeps its raw bytes");
    };
    assert_eq!(
        matches_capture(&fixture, &capture),
        Err(ProcessDiscrepancyError::TerminationDiffers)
    );
    complete_of(&mut capture.invocation).observer_no_deadline = YesNo::Yes;
    assert_eq!(matches_capture(&fixture, &capture), Ok(()));
    capture.output.signal = TerminationSignal::Signalled(SignalNumber(15));
    assert_eq!(
        matches_capture(&fixture, &capture),
        Err(ProcessDiscrepancyError::TerminationDiffers)
    );
    capture.output.signal = TerminationSignal::Exited;
    capture
        .output
        .stderr
        .0
        .extend_from_slice(b"\nstack backtrace\n");
    assert_eq!(
        matches_capture(&fixture, &capture),
        Err(ProcessDiscrepancyError::BytesDiffer)
    );
}
