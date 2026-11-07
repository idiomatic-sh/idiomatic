//! Receipts, archives and source identities refuse every drifted process identity.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::process_failure::errors::ProcessFailureError;
use crate::domain::process_failure::service::group_digest;
use crate::inbound::use_cases::verify_cases::fixtures::{
    process_capture::Fixture,
    process_identity::{ContextProbe, Verdict},
};
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;

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
type Value = Outcome;
type Failure = ();

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
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

/// Calls the scenario on one input.
fn subject(scenario: Input) -> Result<Value, Failure> {
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
    let wrong_binary = format!("\"{}\"", "b".repeat(64));
    let wrong_group = format!("\"{}\"", "c".repeat(64));
    for (pointer, value) in [
        ("/actual_request_root", "\"relative\""),
        ("/process_observation/exit", "124"),
        ("/process_observation/signal", "15"),
        ("/process_observation/observer_no_deadline", "false"),
        (
            "/process_observation/metadata_error",
            "\"identity unavailable\"",
        ),
        (
            "/discrepancy_validation/actual_executable_sha256",
            wrong_binary.as_str(),
        ),
        (
            "/discrepancy_validation/process_observation_group_sha256",
            wrong_group.as_str(),
        ),
        ("/diagnostics", "[]"),
    ] {
        assert_eq!(
            fixture.replay_tampered(0, pointer, value),
            Verdict::Refused,
            "accepted {pointer}"
        );
    }
    assert_eq!(
        fixture.replay_finding(1, "A001", "case.py", 0, 1),
        Verdict::Refused,
        "accepted candidate /diagnostics"
    );
    for (pointer, value) in [
        ("/execution_metadata/availability", "\"unknown\""),
        ("/coverage/unknown_or_unread_rule_file_pairs", "1"),
        ("/case_passed", "false"),
    ] {
        assert_eq!(
            fixture.replay_tampered(1, pointer, value),
            Verdict::Refused,
            "accepted candidate {pointer}"
        );
    }
    assert_eq!(fixture.replay_contract("complete-report"), Verdict::Refused);
}

fn archived_pairs_are_recomputed() {
    let fixture = Fixture::new("identity");
    let mut process = fixture.declared_failure();
    let actual = ArtifactDigest("a".repeat(64));
    group_digest(&process, &actual).unwrap();
    assert_eq!(
        group_digest(&process, &ArtifactDigest("b".repeat(64))),
        Err(ProcessFailureError::NoReproducibleGroup)
    );
    let original = process.observations[1].clone();
    process.observations[1] = process.observations[0].clone();
    assert_eq!(
        group_digest(&process, &actual),
        Err(ProcessFailureError::GroupsShareCaptures)
    );
    process.observations[1] = original;
    process.observations.pop();
    assert_eq!(
        group_digest(&process, &actual),
        Err(ProcessFailureError::NoReproducibleGroup)
    );
    assert_eq!(fixture.replay(&fixture.forged_stdout()), Verdict::Refused);
}

fn raw_invocation_does_not_pin_settings() {
    let fixture = Fixture::new("argv");
    let loaded = fixture.rearmed_invocation("--changed-flag");
    assert!(fixture.refusal(&loaded).contains("argv/environment"));
}

fn unselected_context_stays_bound() {
    let mut probe = ContextProbe::new();
    assert_eq!(
        probe.diagnostic_files(),
        1,
        "diagnostic selection remains independent"
    );
    assert_eq!(probe.check(), Verdict::Accepted);
    probe.write_context("value = 2\n");
    assert_eq!(
        probe.check(),
        Verdict::Refused,
        "unselected context mutation must invalidate evidence"
    );
    probe.write_context("value = 1\n");
    assert_eq!(probe.check(), Verdict::Accepted);
    assert_eq!(
        probe.unlisted_sources(),
        vec![std::path::PathBuf::from("case.py")]
    );
}
