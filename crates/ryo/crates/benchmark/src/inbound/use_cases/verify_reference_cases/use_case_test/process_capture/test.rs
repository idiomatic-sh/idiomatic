//! Real verification capture, boundary archive loading, and strict offline comparison.
use ryo_benchmark_contracts::domain::documents::types::text::value::{ReasonText, YesNo};
use super::fixtures::Fixture;
use crate::application::case_execution::{service::run as run_case, types::Capture};
use crate::application::case_observation::service::write as record;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::discrepancy::types::Contestant;
use crate::outbound::{
    inventory::{interface::InventoryPort as _, types::InvocationDocument},
};
use super::super::process_identity::fixtures::{Observer, complete_of};
use ryo_benchmark_contracts::domain::measurements::types::{SignalNumber, TerminationSignal};
use ryo_benchmark_contracts::domain::receipts::types::CaseKey;
use ryo_benchmark_contracts::domain::requests::types::RuleLabel;
use ryo_benchmark_contracts::domain::workloads::types::benchmark_case_name::value::BenchmarkCaseName;
use serde_json::Value;

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
type Failure = ();

const CASES: &[Case<Input, Outcome, Failure>] = &[
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

fn subject(scenario: Input) -> Result<Outcome, Failure> {
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
fn matches_capture(
    fixture: &Fixture,
    capture: &Capture,
) -> Result<(), crate::application::process_discrepancy::errors::ProcessDiscrepancyError> {
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
            let ryo_benchmark_contracts::domain::documents::types::observation_document::value::ReferenceProcess::Observed(observed) = &captured.records[0].process_observation else {
                panic!("the reference process is observed");
            };
            let ryo_benchmark_contracts::domain::documents::types::observation_document::value::ProcessArtifactsRecord::Attached(pointers) = &observed.artifacts else {
                panic!("the observed process points at artifacts");
            };
            let stdout = &captured.artifacts[std::path::Path::new(&pointers.stdout.path.0)];
            let decoded: Value = serde_json::from_slice(stdout).unwrap();
            assert_eq!(
                decoded["file"],
                fixture
                    .prepared
                    .request
                    .root
                    .join("case.py")
                    .to_str()
                    .unwrap()
            );
            assert!(String::from_utf8_lossy(stdout).contains("日本"));
            if cfg!(unix) {
                assert!(
                    String::from_utf8_lossy(stdout).contains("\\\\"),
                    "JSON encoding must preserve and escape a real backslash in the root"
                );
                assert!(
                    String::from_utf8_lossy(stdout).contains("\\\""),
                    "JSON encoding must escape a real quote in the root"
                );
            }
            let stderr = &captured.artifacts[std::path::Path::new(&pointers.stderr.path.0)];
            assert_eq!(
                stderr,
                format!(
                    "open {}\n",
                    fixture.prepared.request.root.join("case.py").display()
                )
                .as_bytes()
            );
        }
        assert_eq!(
            captured.records[0].diagnostics,
            ryo_benchmark_contracts::domain::documents::types::observation_document::value::ReportedFindings::Unreported
        );
        assert_eq!(
            captured.records[0].raw_parity_matches_intended,
            ryo_benchmark_contracts::domain::documents::types::observation_document::value::IntentMatch::Matched(
                ryo_benchmark_contracts::domain::documents::types::text::value::YesNo::No
            )
        );
        assert_eq!(
            crate::domain::inventory_receipts::service::validate_with_artifacts(
                &[fixture.document.clone()],
                &captured.records,
                &captured.artifacts,
                &captured.invocations
            ),
            Ok(
                crate::domain::inventory_receipts::types::ReceiptCounts {
                    raw_exact: crate::domain::inventory_receipts::types::Tally(0),
                    documented: crate::domain::inventory_receipts::types::Tally(1),
                    contract_only: crate::domain::inventory_receipts::types::Tally(0)
                }
            )
        );
        let audit = crate::application::receipt_validation::service::validate(&fixture.registration(), &captured).unwrap();
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
    let linter = crate::outbound::linter::fixtures::fake::LinterFake::default();
    let execution = Observer(crate::outbound::execution::fixtures::fake::ExecutionFake {
        status: 2,
        ..Default::default()
    });
    let context = crate::outbound::context::factory::local();
    let outcome = run_case(
        &suite,
        &context,
        &linter,
        &execution,
        &fixture.prepared,
        &fixture.rules,
    );
    assert!(outcome.result.is_err());
    let capture = outcome.capture.unwrap();
    assert_eq!(capture.output.stdout.0, b"payload");
    assert!(matches!(capture.invocation, InvocationDocument::Failed(_)));
    let receipt = fixture.root.path().join("failed.jsonl");
    let receipts = crate::outbound::receipts::fixtures::fake::ReceiptsFake {
        case_receipts: crate::outbound::receipts::types::CaseReceipts::Recorded(receipt.clone()),
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
        Some(&capture),
    )
    .unwrap();
    let loaded = crate::outbound::inventory::factory::local()
        .observations(&receipt)
        .unwrap();
    assert_eq!(loaded.artifacts.len(), 3);
    assert_eq!(
        loaded.records[0].case_passed,
        YesNo::No
    );
    assert!(matches!(
        loaded.records[0].error,
        ryo_benchmark_contracts::domain::documents::types::observation_document::value::RecordedReason::Stated(_)
    ));
}

fn terminations_are_not_expected_failures() {
    let fixture = Fixture::new("termination");
    let linter = crate::outbound::linter::fixtures::fake::LinterFake::default();
    let context = crate::outbound::context::factory::local();
    for status in [0, 1] {
        let execution = Observer(crate::outbound::execution::fixtures::fake::ExecutionFake {
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
        assert!(outcome.result.is_ok());
        assert!(
            outcome.capture.is_none(),
            "a decoded report cannot satisfy a process expectation"
        );
    }
    let bounded = crate::outbound::execution::fixtures::fake::ExecutionFake {
        status: 2,
        ..Default::default()
    };
    let mut capture = run_case(
        &fixture.suite,
        &context,
        &linter,
        &bounded,
        &fixture.prepared,
        &fixture.rules,
    )
    .capture
    .unwrap();
    assert!(matches_capture(&fixture, &capture).is_err());
    complete_of(&mut capture.invocation).observer_no_deadline = YesNo::Yes;
    assert!(matches_capture(&fixture, &capture).is_ok());
    capture.output.signal = TerminationSignal::Signalled(ryo_benchmark_contracts::domain::measurements::types::SignalNumber(15));
    assert!(matches_capture(&fixture, &capture).is_err());
    capture.output.signal = TerminationSignal::Exited;
    capture
        .output
        .stderr
        .0
        .extend_from_slice(b"\nstack backtrace\n");
    assert!(matches_capture(&fixture, &capture).is_err());
}
