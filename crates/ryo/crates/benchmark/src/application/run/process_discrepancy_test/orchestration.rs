//! Exercise the actual verifier orchestration and its fail-closed observer gate.
use super::*;
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
            .any(|arg| arg == crate::application::run::constants::CANDIDATE)
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
    let fixture = fixtures::Fixture::new(crate::application::run::constants::ORCHESTRATION);
    let mut suite = fixture.suite.clone();
    let mut prepared = fixture.prepared.clone();
    prepared
        .reference_discrepancy
        .as_mut()
        .unwrap()
        .evidence
        .push(DiscrepancyEvidence::Language {
            path: crate::application::run::constants::SEMANTIC_AUTHORITY_TXT.into(),
            sha256: ArtifactDigest(crate::application::run::constants::F.repeat(64)),
            kind: DiscrepancyEvidenceKind::LanguageSpec,
            language_version: crate::application::run::constants::TOY_V1.into(),
            target_configuration: crate::application::run::constants::PYTHON3_10.into(),
            version_rationale:
                crate::application::run::constants::TOY_FIXTURE_CONTRACT_INDEPENDENT_OF_PROCESS
                    .into(),
        });
    suite.prepared = vec![prepared];
    let reference = crate::outbound::linter::fixtures::fake::LinterFake::default();
    let candidate = crate::outbound::linter::fixtures::fake::LinterFake {
        arguments: vec![crate::application::run::constants::CANDIDATE.into()],
        ..Default::default()
    };
    let bounded = crate::outbound::execution::fixtures::fake::ExecutionFake::default();
    assert!(
        service::verify_prepared_shard(
            &suite,
            &reference,
            &candidate,
            &bounded,
            &fixture.rules,
            suite.prepared.clone(),
            ryo_benchmark_contracts::domain::requests::types::CaseSelection::default()
        )
        .unwrap_err()
        .to_string()
        .contains(crate::application::run::constants::NO_DEADLINE)
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
        inventory_receipts::validate_with_artifacts(
            &[fixture.case.clone()],
            &captured.records,
            &captured.artifacts
        ),
        Ok((0, 1, 0))
    );
    suite.prepared[0].reference_discrepancy = None;
    std::fs::remove_file(suite.case_receipt.as_ref().unwrap()).unwrap();
    assert!(
        service::verify_prepared_shard(
            &suite,
            &reference,
            &candidate,
            &PairExecution,
            &fixture.rules,
            suite.prepared.clone(),
            ryo_benchmark_contracts::domain::requests::types::CaseSelection::default()
        )
        .is_err(),
        "an untagged process failure remains a failure"
    );
}
