//! Exercise the actual verifier orchestration and its fail-closed observer gate.
use super::*;
use crate::application::run::constants;
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::workloads::types::discrepancy_evidence::value::{
    DiscrepancyEvidence, DiscrepancyEvidenceKind,
};

/// One orchestration substitute distinguishes actual provider commands, without deciding comparison policy.
struct PairExecution;
impl ExecutionPort for PairExecution {
    fn profile(&self) -> ExecutionProfile {
        ExecutionProfile::Wall
    }
    fn observes_completed_processes_without_deadline(&self) -> bool {
        true
    }
    fn run(
        &self,
        invocation: Invocation,
    ) -> Result<ProcessOutput, ryo_benchmark_contracts::domain::reports::errors::ReportsError> {
        let status = if invocation
            .arguments
            .iter()
            .any(|arg| arg == constants::CANDIDATE)
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

#[test]
fn production_verifier_rejects_unavailable_observers_and_accepts_only_the_declared_reference_failure()
 {
    let fixture = fixtures::Fixture::new(constants::ORCHESTRATION);
    let mut suite = fixture.suite.clone();
    let mut prepared = fixture.prepared.clone();
    prepared
        .reference_discrepancy
        .as_mut()
        .unwrap()
        .evidence
        .push(DiscrepancyEvidence::Language {
            path: constants::SEMANTIC_AUTHORITY_TXT.into(),
            sha256: ArtifactDigest(constants::F.repeat(64)),
            kind: DiscrepancyEvidenceKind::LanguageSpec,
            language_version: constants::TOY_V1.into(),
            target_configuration: constants::PYTHON3_10.into(),
            version_rationale: constants::TOY_FIXTURE_CONTRACT_INDEPENDENT_OF_PROCESS.into(),
        });
    suite.prepared = vec![prepared];
    let reference = crate::outbound::linter::fixtures::fake::LinterFake::default();
    let candidate = crate::outbound::linter::fixtures::fake::LinterFake {
        arguments: vec![constants::CANDIDATE.into()],
        ..Default::default()
    };
    let bounded = crate::outbound::execution::fixtures::fake::ExecutionFake::default();
    assert!(
        service::verify_prepared_shard(
            &suite,
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
        .contains(constants::NO_DEADLINE)
    );
    assert!(
        bounded.history.lock().unwrap().is_empty(),
        "unsupported observer must refuse before launching either contestant"
    );
    std::fs::remove_file(suite.case_receipt.as_ref().unwrap()).unwrap();
    assert_eq!(
        service::verify_prepared_shard(
            &suite,
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
        crate::application::inventory_receipts::service::validate_with_artifacts(
            &[fixture.document.clone()],
            &captured.records,
            &captured.artifacts,
            &captured.invocations
        ),
        Ok(
            crate::application::inventory_receipts::types::ReceiptCounts {
                raw_exact: crate::application::inventory_receipts::types::Tally(0),
                documented: crate::application::inventory_receipts::types::Tally(1),
                contract_only: crate::application::inventory_receipts::types::Tally(0)
            }
        )
    );
    suite.prepared[0].reference_discrepancy = None;
    std::fs::remove_file(suite.case_receipt.as_ref().unwrap()).unwrap();
    assert!(
        service::verify_prepared_shard(
            &suite,
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
