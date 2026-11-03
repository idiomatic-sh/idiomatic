//! Exercise the actual verifier orchestration and its fail-closed observer gate.
use super::*;

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
    ) -> Result<ProcessOutput, crate::domain::diagnostics::errors::DiagnosticsError> {
        let status = if invocation.arguments.iter().any(|arg| arg == "candidate") {
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
    let fixture = fixtures::Fixture::new("orchestration");
    let mut suite = fixture.suite.clone();
    let mut prepared = fixture.prepared.clone();
    prepared
        .reference_discrepancy
        .as_mut()
        .unwrap()
        .evidence
        .push(DiscrepancyEvidence {
            path: "semantic-authority.txt".into(),
            sha256: ArtifactDigest("f".repeat(64)),
            kind: DiscrepancyEvidenceKind::LanguageSpec,
            language_version: "toy-v1".into(),
            target_configuration: "python3.10".into(),
            version_rationale: "toy fixture contract independent of process output".into(),
        });
    suite.prepared = vec![prepared];
    let reference = crate::outbound::linter::fixtures::fake::LinterFake::default();
    let candidate = crate::outbound::linter::fixtures::fake::LinterFake {
        arguments: vec!["candidate".into()],
        ..Default::default()
    };
    let bounded = crate::outbound::execution::fixtures::fake::ExecutionFake::default();
    assert!(
        service::verify_shard(
            &suite,
            &reference,
            &candidate,
            &bounded,
            &fixture.rules,
            fixture.root.path(),
            service::CaseSelection::default()
        )
        .unwrap_err()
        .to_string()
        .contains("no-deadline")
    );
    assert!(
        bounded.history.lock().unwrap().is_empty(),
        "unsupported observer must refuse before launching either contestant"
    );
    std::fs::remove_file(suite.case_receipt.as_ref().unwrap()).unwrap();
    assert_eq!(
        service::verify_shard(
            &suite,
            &reference,
            &candidate,
            &PairExecution,
            &fixture.rules,
            fixture.root.path(),
            service::CaseSelection::default()
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
        service::verify_shard(
            &suite,
            &reference,
            &candidate,
            &PairExecution,
            &fixture.rules,
            fixture.root.path(),
            service::CaseSelection::default()
        )
        .is_err(),
        "an untagged process failure remains a failure"
    );
}
