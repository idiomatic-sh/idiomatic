//! Real verification capture, boundary archive loading, and strict offline comparison.
use super::*;
use crate::outbound::{
    execution::interface::ExecutionPort, inventory::interface::InventoryPort as _,
    suite::interface::SuitePort as _,
};
use ryo_benchmark_contracts::domain::measurements::types::*;
use ryo_benchmark_contracts::domain::reports::types::*;
use ryo_benchmark_contracts::domain::workloads::types::*;
use serde_json::{Value, json};
use std::collections::BTreeMap;
mod fixtures;
mod orchestration;

/// Explicit no-deadline capability; the underlying fixture retains actual invocation history.
struct Observer(crate::outbound::execution::fixtures::fake::ExecutionFake);
impl ExecutionPort for Observer {
    fn profile(&self) -> ExecutionProfile {
        self.0.profile()
    }
    fn observes_completed_processes_without_deadline(&self) -> bool {
        true
    }
    fn run(
        &self,
        invocation: Invocation,
    ) -> Result<ProcessOutput, ryo_benchmark_contracts::domain::reports::errors::ReportsError> {
        self.0.run(invocation)
    }
}

#[test]
fn two_tool_neutral_providers_capture_archive_and_validate_without_a_report() {
    for provider in ["provider-one", "provider-two"] {
        let fixture = if provider == "provider-one" {
            fixtures::Fixture::new(provider)
        } else {
            fixtures::Fixture::with_paths(provider)
        };
        let captured = fixture.load();
        assert_eq!(captured.records.len(), 2);
        assert_eq!(captured.artifacts.len(), 3);
        if provider == "provider-two" {
            let pointers = captured.records[0]
                .process_observation
                .as_ref()
                .and_then(|process| process.artifacts.as_ref())
                .unwrap();
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
        assert!(captured.records[0].diagnostics.is_none());
        assert_eq!(
            captured.records[0].raw_parity_matches_intended,
            Some(crate::outbound::inventory::types::text::YesNo::No)
        );
        assert_eq!(
            crate::application::inventory_receipts::service::validate_with_artifacts(
                &[fixture.document.clone()],
                &captured.records,
                &captured.artifacts
            ),
            Ok(
                crate::application::inventory_receipts::types::ReceiptCounts {
                    raw_exact: crate::application::inventory_receipts::types::Tally(0),
                    documented: crate::application::inventory_receipts::types::Tally(1),
                    contract_only: crate::application::inventory_receipts::types::Tally(0)
                }
            )
        );
        let audit = receipt_validation::validate(&fixture.registration(), &captured).unwrap();
        assert!(audit.errors.is_empty());
        assert_eq!(audit.verified, vec![("A001".into(), provider.into())]);
        assert_eq!(
            (
                audit.raw_exact,
                audit.documented_discrepancies,
                audit.contract_only
            ),
            (0, 1, 0)
        );
    }
}

#[test]
fn offline_replay_rejects_every_identity_and_candidate_completeness_mutation() {
    let fixture = fixtures::Fixture::new("consumer");
    for (pointer, value) in [
        ("/actual_request_root", json!("relative")),
        ("/process_observation/exit", json!(124)),
        ("/process_observation/signal", json!(15)),
        ("/process_observation/observer_no_deadline", json!(false)),
        (
            "/process_observation/metadata_error",
            json!("identity unavailable"),
        ),
        (
            "/discrepancy_validation/actual_executable_sha256",
            json!("b".repeat(64)),
        ),
        (
            "/discrepancy_validation/process_observation_group_sha256",
            json!("c".repeat(64)),
        ),
        ("/diagnostics", json!([])),
    ] {
        let refused = fixture.tampered(0, pointer, value).is_none_or(|captured| {
            crate::application::inventory_receipts::service::validate_with_artifacts(
                &[fixture.document.clone()],
                &captured.records,
                &captured.artifacts,
            )
            .is_err()
        });
        assert!(refused, "accepted {pointer}");
    }
    for (pointer, value) in [
        (
            "/diagnostics",
            json!([{"rule":"A001","file":"case.py","start":0,"end":1}]),
        ),
        ("/execution_metadata/availability", json!("unknown")),
        ("/coverage/unknown_or_unread_rule_file_pairs", json!(1)),
        ("/case_passed", json!(false)),
    ] {
        let refused = fixture.tampered(1, pointer, value).is_none_or(|captured| {
            crate::application::inventory_receipts::service::validate_with_artifacts(
                &[fixture.document.clone()],
                &captured.records,
                &captured.artifacts,
            )
            .is_err()
        });
        assert!(refused, "accepted candidate {pointer}");
    }
    let case =
        fixture.document_with(|case| case["expectation_contract"] = json!("complete-report"));
    assert!(
        crate::application::inventory_receipts::service::validate_with_artifacts(
            &[case],
            &fixture.load().records,
            &fixture.load().artifacts
        )
        .is_err()
    );
}

#[test]
fn archived_pairs_are_required_for_each_actual_binary_and_raw_bytes_are_recomputed() {
    let fixture = fixtures::Fixture::new("identity");
    let mut process = process_discrepancy::convert(
        serde_json::from_value(fixture.case["reference_discrepancy"]["process"].clone()).unwrap(),
    )
    .unwrap();
    assert!(process_discrepancy::group_digest(&process, &"a".repeat(64)).is_ok());
    assert!(process_discrepancy::group_digest(&process, &"b".repeat(64)).is_err());
    let original = process.observations[1].clone();
    process.observations[1] = process.observations[0].clone();
    assert!(process_discrepancy::group_digest(&process, &"a".repeat(64)).is_err());
    process.observations[1] = original;
    process.observations.pop();
    assert!(process_discrepancy::group_digest(&process, &"a".repeat(64)).is_err());
    let mut captured = fixture.load();
    let path = std::path::PathBuf::from(
        &captured.records[0]
            .process_observation
            .as_ref()
            .and_then(|process| process.artifacts.as_ref())
            .unwrap()
            .stdout
            .path
            .0,
    );
    captured.artifacts.insert(path, b"forged".to_vec());
    assert!(
        crate::application::inventory_receipts::service::validate_with_artifacts(
            &[fixture.document.clone()],
            &captured.records,
            &captured.artifacts
        )
        .is_err()
    );
}

#[test]
fn metadata_failure_preserves_process_bytes_and_cannot_be_admitted() {
    let fixture = fixtures::Fixture::new("metadata");
    let mut suite = fixture.suite.clone();
    suite.artifact_digests.clear();
    let linter = crate::outbound::linter::fixtures::fake::LinterFake::default();
    let execution = Observer(crate::outbound::execution::fixtures::fake::ExecutionFake {
        status: 2,
        ..Default::default()
    });
    let outcome = case_execution::run(
        &suite,
        &linter,
        &execution,
        &fixture.prepared,
        &fixture.rules,
    );
    assert!(outcome.result.is_err());
    let capture = outcome.capture.unwrap();
    assert_eq!(capture.output.stdout.0, b"payload");
    assert!(capture.invocation["metadata_error"].is_string());
    let receipt = fixture.root.path().join("failed.jsonl");
    suite.case_receipt = Some(receipt.clone());
    case_observations::write(
        &suite,
        &fixture.prepared,
        "reference",
        None,
        Some("metadata unavailable"),
        false,
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
        Some(crate::outbound::inventory::types::text::YesNo::No)
    );
    assert!(loaded.records[0].error.is_some());
}

#[test]
fn decoded_success_deadline_observer_signals_and_extra_backtrace_are_not_expected_failures() {
    let fixture = fixtures::Fixture::new("termination");
    let linter = crate::outbound::linter::fixtures::fake::LinterFake::default();
    let proof = fixture.proof();
    for status in [0, 1] {
        let execution = Observer(crate::outbound::execution::fixtures::fake::ExecutionFake {
            status,
            ..Default::default()
        });
        let outcome = case_execution::run(
            &fixture.suite,
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
    let mut capture = case_execution::run(
        &fixture.suite,
        &linter,
        &bounded,
        &fixture.prepared,
        &fixture.rules,
    )
    .capture
    .unwrap();
    assert!(
        process_discrepancy::matches(
            &fixture.prepared,
            &capture,
            &proof,
            &fixture.suite,
            &fixture.rules
        )
        .is_err()
    );
    capture.invocation["observer_no_deadline"] = json!(true);
    assert!(
        process_discrepancy::matches(
            &fixture.prepared,
            &capture,
            &proof,
            &fixture.suite,
            &fixture.rules
        )
        .is_ok()
    );
    capture.output.signal = Some(15);
    assert!(
        process_discrepancy::matches(
            &fixture.prepared,
            &capture,
            &proof,
            &fixture.suite,
            &fixture.rules
        )
        .is_err()
    );
    capture.output.signal = None;
    capture
        .output
        .stderr
        .0
        .extend_from_slice(b"\nstack backtrace\n");
    assert!(
        process_discrepancy::matches(
            &fixture.prepared,
            &capture,
            &proof,
            &fixture.suite,
            &fixture.rules
        )
        .is_err()
    );
}

#[test]
fn changing_raw_invocation_and_its_digest_does_not_establish_pinned_settings() {
    use sha2::Digest as _;
    let fixture = fixtures::Fixture::new("argv");
    let mut captured = fixture.load();
    let pointer = captured.records[0]
        .process_observation
        .as_mut()
        .and_then(|process| process.artifacts.as_mut())
        .map(|artifacts| &mut artifacts.invocation)
        .unwrap();
    let path = std::path::PathBuf::from(&pointer.path.0);
    let mut invocation: Value = serde_json::from_slice(&captured.artifacts[&path]).unwrap();
    invocation["arguments"] = json!(["--changed-flag"]);
    let bytes = serde_json::to_vec(&invocation).unwrap();
    pointer.sha256 = crate::outbound::inventory::types::text::Sha256Text(format!(
        "{:x}",
        sha2::Sha256::digest(&bytes)
    ));
    captured.artifacts.insert(path, bytes);
    assert!(
        crate::application::inventory_receipts::service::validate_with_artifacts(
            &[fixture.document.clone()],
            &captured.records,
            &captured.artifacts
        )
        .unwrap_err()
        .to_string()
        .contains("argv/environment")
    );
}
