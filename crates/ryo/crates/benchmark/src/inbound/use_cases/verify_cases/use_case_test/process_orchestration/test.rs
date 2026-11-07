//! Exercise the actual verifier orchestration and its fail-closed observer gate.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::inventory_receipts::types::{ReceiptCounts, Tally};
use crate::inbound::use_cases::verify_cases::fixtures::{
    process_capture::Fixture,
    shard::{Refusal, category, forget_receipts, verify_shard},
};
use crate::outbound::execution::fixtures::fake::ExecutionFake;
use crate::outbound::execution::interface::ExecutionPort;
use crate::outbound::linter::fixtures::fake::LinterFake;
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::measurements::types::{ExecutionProfile, Invocation, ProcessOutput};
use ryo_benchmark_contracts::domain::workloads::types::PreparedReferenceBehavior;
use ryo_benchmark_contracts::domain::workloads::types::discrepancy_evidence::value::{
    DiscrepancyEvidence, DiscrepancyEvidenceKind, LanguageVersion, ParserTarget, Rationale,
};

const ORCHESTRATION: &str = "orchestration";
const SEMANTIC_AUTHORITY_TXT: &str = "semantic-authority.txt";
const LETTER: &str = "f";
const TOY_LANGUAGE: &str = "toy";
const TARGET_CONFIGURATION: &str = "python3.10";
const VERSION_RATIONALE: &str = "toy fixture contract independent of process output";
const NO_DEADLINE: &str = "no-deadline";
const CANDIDATE: &str = "candidate";

/// What the orchestration establishes once its assertions hold.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Holds,
}

type Input = ();
type Value = Outcome;
type Failure = ();

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("the production verifier rejects unavailable observers and accepts only the declared reference failure"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(Outcome::Holds),
}];

/// Calls the verifier on one input.
fn subject(_input: Input) -> Result<Value, Failure> {
    production_verifier_gates_observers_and_accepts_only_the_declared_reference_failure();
    Ok(Outcome::Holds)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

/// One orchestration substitute distinguishes actual provider commands, without deciding comparison policy.
struct PairExecution;
impl ExecutionPort for PairExecution {
    fn profile(&self) -> ExecutionProfile {
        ExecutionProfile::Wall
    }
    fn exit_observation(&self) -> crate::outbound::execution::types::ExitObservationMode {
        crate::outbound::execution::types::ExitObservationMode::Direct
    }
    fn run(
        &self,
        invocation: Invocation,
    ) -> Result<ProcessOutput, ryo_benchmark_contracts::domain::reports::errors::ReportsError> {
        let status = if invocation
            .arguments
            .iter()
            .any(|arg| arg == CANDIDATE)
        {
            0
        } else {
            2
        };
        ExecutionFake {
            status,
            ..Default::default()
        }
        .run(invocation)
    }
}

fn production_verifier_gates_observers_and_accepts_only_the_declared_reference_failure() {
    let fixture = Fixture::new(ORCHESTRATION);
    let mut suite = fixture.suite.clone();
    let mut prepared = fixture.prepared.clone();
    let PreparedReferenceBehavior::Differs(discrepancy) = &mut prepared.reference_discrepancy else {
        unreachable!("the fixture documents a reference failure")
    };
    discrepancy.evidence.push(DiscrepancyEvidence::Language {
        path: SEMANTIC_AUTHORITY_TXT.into(),
        sha256: ArtifactDigest(LETTER.repeat(64)),
        kind: DiscrepancyEvidenceKind::LanguageSpec,
        language_version: LanguageVersion(TOY_LANGUAGE.into()),
        target_configuration: ParserTarget(TARGET_CONFIGURATION.into()),
        version_rationale: Rationale(VERSION_RATIONALE.into()),
    });
    suite.prepared = vec![prepared];
    let reference = LinterFake::default();
    let candidate = LinterFake {
        arguments: vec![CANDIDATE.into()],
        ..Default::default()
    };
    let bounded = ExecutionFake::default();
    assert!(
        verify_shard(
            &suite,
            &fixture.receipts,
            &reference,
            &candidate,
            &bounded,
            &fixture.rules,
        )
        .unwrap_err()
        .to_string()
        .contains(NO_DEADLINE)
    );
    assert!(
        bounded.history.lock().unwrap().is_empty(),
        "unsupported observer must refuse before launching either contestant"
    );
    forget_receipts(&fixture.receipts);
    assert_eq!(
        verify_shard(
            &suite,
            &fixture.receipts,
            &reference,
            &candidate,
            &PairExecution,
            &fixture.rules,
        )
        .unwrap(),
        1
    );
    let loaded = fixture.load();
    assert_eq!(
        crate::domain::inventory_receipts::service::validate_with_artifacts(
            &[fixture.document.clone()],
            &loaded.records,
            &loaded.artifacts,
            &loaded.invocations
        ),
        Ok(ReceiptCounts {
            raw_exact: Tally(0),
            documented: Tally(1),
            contract_only: Tally(0)
        })
    );
    suite.prepared[0].reference_discrepancy = PreparedReferenceBehavior::Agrees;
    forget_receipts(&fixture.receipts);
    let untagged = verify_shard(
        &suite,
        &fixture.receipts,
        &reference,
        &candidate,
        &PairExecution,
        &fixture.rules,
    )
    .unwrap_err();
    assert_eq!(
        category(&untagged.to_string()),
        Refusal::ReferenceStartup,
        "an untagged process failure remains a failure"
    );
}
