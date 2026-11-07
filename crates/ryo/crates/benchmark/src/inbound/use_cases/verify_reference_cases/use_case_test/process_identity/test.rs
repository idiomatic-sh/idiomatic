//! Receipts, archives and source identities refuse every drifted process identity.
use super::super::process_capture::fixtures::Fixture;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::requests::types::StagedSources;
use serde_json::{Value, json};

/// What an identity scenario establishes once its assertions hold.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Holds,
}

/// Which drifted identity a row exercises.
#[derive(Clone, Copy)]
enum Scenario {
    ReplayRejectsMutations,
    ArchivedPairsAreRecomputed,
    RawInvocationDoesNotPinSettings,
    UnselectedContextStaysBound,
}

type Input = Scenario;
type Failure = ();

const CASES: &[Case<Input, Outcome, Failure>] = &[
    Case {
        name: CaseName("offline replay rejects every identity and candidate completeness mutation"),
        word: Word::Succeeds,
        input: || Scenario::ReplayRejectsMutations,
        expect: Expect::Value(Outcome::Holds),
    },
    Case {
        name: CaseName("archived pairs are required for each actual binary and raw bytes are recomputed"),
        word: Word::Succeeds,
        input: || Scenario::ArchivedPairsAreRecomputed,
        expect: Expect::Value(Outcome::Holds),
    },
    Case {
        name: CaseName("changing the raw invocation and its digest does not establish pinned settings"),
        word: Word::Succeeds,
        input: || Scenario::RawInvocationDoesNotPinSettings,
        expect: Expect::Value(Outcome::Holds),
    },
    Case {
        name: CaseName("an unselected context source remains bound to the discrepancy identity"),
        word: Word::Succeeds,
        input: || Scenario::UnselectedContextStaysBound,
        expect: Expect::Value(Outcome::Holds),
    },
];

fn subject(scenario: Input) -> Result<Outcome, Failure> {
    match scenario {
        Scenario::ReplayRejectsMutations => replay_rejects_mutations(),
        Scenario::ArchivedPairsAreRecomputed => archived_pairs_are_recomputed(),
        Scenario::RawInvocationDoesNotPinSettings => raw_invocation_does_not_pin_settings(),
        Scenario::UnselectedContextStaysBound => unselected_context_stays_bound(),
    }
    Ok(Outcome::Holds)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

fn replay_rejects_mutations() {
    let fixture = Fixture::new("consumer");
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
            crate::domain::inventory_receipts::service::validate_with_artifacts(
                &[fixture.document.clone()],
                &captured.records,
                &captured.artifacts,
                &captured.invocations,
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
            crate::domain::inventory_receipts::service::validate_with_artifacts(
                &[fixture.document.clone()],
                &captured.records,
                &captured.artifacts,
                &captured.invocations,
            )
            .is_err()
        });
        assert!(refused, "accepted candidate {pointer}");
    }
    let case =
        fixture.document_with(|case| case["expectation_contract"] = json!("complete-report"));
    assert!(
        crate::domain::inventory_receipts::service::validate_with_artifacts(
            &[case],
            &fixture.load().records,
            &fixture.load().artifacts,
            &fixture.load().invocations
        )
        .is_err()
    );
}

fn archived_pairs_are_recomputed() {
    let fixture = Fixture::new("identity");
    let mut process = crate::inbound::use_cases::verify_reference_cases::use_case_test::process_identity::fixtures::process_failure(&fixture.case["reference_discrepancy"]["process"]);
    assert!(
        crate::domain::process_failure::service::group_digest(
            &process,
            &ArtifactDigest("a".repeat(64))
        )
        .is_ok()
    );
    assert!(
        crate::domain::process_failure::service::group_digest(
            &process,
            &ArtifactDigest("b".repeat(64))
        )
        .is_err()
    );
    let original = process.observations[1].clone();
    process.observations[1] = process.observations[0].clone();
    assert!(
        crate::domain::process_failure::service::group_digest(
            &process,
            &ArtifactDigest("a".repeat(64))
        )
        .is_err()
    );
    process.observations[1] = original;
    process.observations.pop();
    assert!(
        crate::domain::process_failure::service::group_digest(
            &process,
            &ArtifactDigest("a".repeat(64))
        )
        .is_err()
    );
    let mut captured = fixture.load();
    let ryo_benchmark_contracts::domain::documents::types::observation_document::value::ReferenceProcess::Observed(observed) = &captured.records[0].process_observation else {
        panic!("the reference process is observed");
    };
    let ryo_benchmark_contracts::domain::documents::types::observation_document::value::ProcessArtifactsRecord::Attached(artifacts) = &observed.artifacts else {
        panic!("the observed process points at artifacts");
    };
    let path = std::path::PathBuf::from(&artifacts.stdout.path.0);
    captured.artifacts.insert(path, b"forged".to_vec());
    assert!(
        crate::domain::inventory_receipts::service::validate_with_artifacts(
            &[fixture.document.clone()],
            &captured.records,
            &captured.artifacts,
            &captured.invocations
        )
        .is_err()
    );
}

fn raw_invocation_does_not_pin_settings() {
    use sha2::Digest as _;
    let fixture = Fixture::new("argv");
    let mut captured = fixture.load();
    let ryo_benchmark_contracts::domain::documents::types::observation_document::value::ReferenceProcess::Observed(observed) = &mut captured.records[0].process_observation else {
        panic!("the reference process is observed");
    };
    let ryo_benchmark_contracts::domain::documents::types::observation_document::value::ProcessArtifactsRecord::Attached(artifacts) = &mut observed.artifacts else {
        panic!("the observed process points at artifacts");
    };
    let pointer = &mut artifacts.invocation;
    let path = std::path::PathBuf::from(&pointer.path.0);
    let mut invocation: Value = serde_json::from_slice(&captured.artifacts[&path]).unwrap();
    invocation["arguments"] = json!(["--changed-flag"]);
    let bytes = serde_json::to_vec(&invocation).unwrap();
    pointer.sha256 = ryo_benchmark_contracts::domain::documents::types::text::value::Sha256Text(format!(
        "{:x}",
        sha2::Sha256::digest(&bytes)
    ));
    captured.invocations.insert(
        path.clone(),
        crate::outbound::inventory::factory::local()
            .decode_invocation(&bytes)
            .unwrap(),
    );
    captured.artifacts.insert(path, bytes);
    assert!(
        crate::domain::inventory_receipts::service::validate_with_artifacts(
            &[fixture.document.clone()],
            &captured.records,
            &captured.artifacts,
            &captured.invocations
        )
        .unwrap_err()
        .to_string()
        .contains("argv/environment")
    );
}

fn unselected_context_stays_bound() {
    use crate::outbound::suite::{
        interface::SuitePort as _,
        types::{ArtifactPath, discrepancy::CaseArtifacts},
    };
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("case.py"), "import local\n").unwrap();
    std::fs::write(root.path().join("local.py"), "value = 1\n").unwrap();
    let mut prepared = crate::domain::discrepancy::fixtures::case();
    prepared.request.root = root.path().into();
    prepared.request.staged_sources = StagedSources::Listed(
        vec!["case.py", "local.py"]
            .into_iter()
            .map(|path| ryo::linter::RelativePath(path.into()))
            .collect(),
    );
    let files = crate::domain::suite::service::materialized_sources(&prepared.request);
    assert_eq!(
        prepared.request.files.len(),
        1,
        "diagnostic selection remains independent"
    );
    let suite = crate::outbound::suite::factory::local();
    let digest = suite
        .case_source_digest(
            ArtifactPath(root.path()),
            crate::outbound::suite::types::ArtifactSet(&files),
        )
        .unwrap()
        .0;
    let check = || {
        suite.verify_discrepancy_artifacts(CaseArtifacts {
            profile: root.path(),
            root: root.path(),
            files: &files,
            source_sha256: &digest,
            file_metadata_files: &[],
            file_metadata_sha256: None,
            evidence: &[],
        })
    };
    check().unwrap();
    std::fs::write(root.path().join("local.py"), "value = 2\n").unwrap();
    assert!(
        check().is_err(),
        "unselected context mutation must invalidate evidence"
    );
    std::fs::write(root.path().join("local.py"), "value = 1\n").unwrap();
    check().unwrap();
    prepared.request.staged_sources = StagedSources::Unlisted;
    assert_eq!(
        crate::domain::suite::service::materialized_sources(&prepared.request),
        vec![std::path::PathBuf::from("case.py")]
    );
}
