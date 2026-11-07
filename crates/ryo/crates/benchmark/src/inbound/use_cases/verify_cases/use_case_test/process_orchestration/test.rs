//! Exercise the actual verifier orchestration and its fail-closed observer gate.
use super::super::super::service::verify_prepared_shard;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::inbound::use_cases::verify_reference_cases::use_case_test::process_capture::fixtures::Fixture;
use crate::outbound::execution::interface::ExecutionPort;
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::measurements::types::{ExecutionProfile, Invocation, ProcessOutput};
use ryo_benchmark_contracts::domain::workloads::types::discrepancy_evidence::value::{
    DiscrepancyEvidence, DiscrepancyEvidenceKind,
};

const ORCHESTRATION: &str = "orchestration";
const SEMANTIC_AUTHORITY_TXT: &str = "semantic-authority.txt";
const LETTER: &str = "f";
const TOY_LANGUAGE: &str = "toy-v1";
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
type Failure = ();

const CASES: &[Case<Input, Outcome, Failure>] = &[Case {
    name: CaseName("the production verifier rejects unavailable observers and accepts only the declared reference failure"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(Outcome::Holds),
}];

fn subject(_input: Input) -> Result<Outcome, Failure> {
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
        crate::outbound::execution::fixtures::fake::ExecutionFake {
            status,
            ..Default::default()
        }
        .run(invocation)
    }
}

/// The receipt file a recording fixture appends to.
fn receipt_path(receipts: &crate::outbound::receipts::fixtures::fake::ReceiptsFake) -> std::path::PathBuf {
    match &receipts.case_receipts {
        crate::outbound::receipts::types::CaseReceipts::Recorded(path) => path.clone(),
        crate::outbound::receipts::types::CaseReceipts::Unrecorded => unreachable!("fixture records receipts"),
    }
}

fn production_verifier_gates_observers_and_accepts_only_the_declared_reference_failure() {
    let fixture = Fixture::new(ORCHESTRATION);
    let context = crate::outbound::context::factory::local();
    let mut suite = fixture.suite.clone();
    let mut prepared = fixture.prepared.clone();
    prepared
        .reference_discrepancy
        .as_mut()
        .unwrap()
        .evidence
        .push(DiscrepancyEvidence::Language {
            path: SEMANTIC_AUTHORITY_TXT.into(),
            sha256: ArtifactDigest(LETTER.repeat(64)),
            kind: DiscrepancyEvidenceKind::LanguageSpec,
            language_version: TOY_LANGUAGE.into(),
            target_configuration: TARGET_CONFIGURATION.into(),
            version_rationale: VERSION_RATIONALE.into(),
        });
    suite.prepared = vec![prepared];
    let reference = crate::outbound::linter::fixtures::fake::LinterFake::default();
    let candidate = crate::outbound::linter::fixtures::fake::LinterFake {
        arguments: vec![CANDIDATE.into()],
        ..Default::default()
    };
    let bounded = crate::outbound::execution::fixtures::fake::ExecutionFake::default();
    assert!(
        verify_prepared_shard(
            &suite,
            &fixture.receipts,
            &context,
            &reference,
            &candidate,
            &bounded,
            &crate::outbound::inventory::factory::local(),
            &fixture.rules,
            suite.prepared.clone(),
            ryo_benchmark_contracts::domain::requests::types::CaseSelection::default()
        )
        .unwrap_err()
        .to_string()
        .contains(NO_DEADLINE)
    );
    assert!(
        bounded.history.lock().unwrap().is_empty(),
        "unsupported observer must refuse before launching either contestant"
    );
    std::fs::remove_file(receipt_path(&fixture.receipts)).unwrap();
    assert_eq!(
        verify_prepared_shard(
            &suite,
            &fixture.receipts,
            &context,
            &reference,
            &candidate,
            &PairExecution,
            &crate::outbound::inventory::factory::local(),
            &fixture.rules,
            suite.prepared.clone(),
            ryo_benchmark_contracts::domain::requests::types::CaseSelection::default()
        )
        .unwrap()
        .0,
        1
    );
    let captured = fixture.load();
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
    suite.prepared[0].reference_discrepancy = ryo_benchmark_contracts::domain::workloads::types::PreparedReferenceBehavior::Agrees;
    std::fs::remove_file(receipt_path(&fixture.receipts)).unwrap();
    assert!(
        verify_prepared_shard(
            &suite,
            &fixture.receipts,
            &context,
            &reference,
            &candidate,
            &PairExecution,
            &crate::outbound::inventory::factory::local(),
            &fixture.rules,
            suite.prepared.clone(),
            ryo_benchmark_contracts::domain::requests::types::CaseSelection::default()
        )
        .is_err(),
        "an untagged process failure remains a failure"
    );
}
