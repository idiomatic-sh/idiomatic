//! What the evidence envelope check accepts and refuses, one row of `CASES` for each input.
use super::verify_requirement;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::suite::errors::SuiteError;
use crate::outbound::suite::types::EvidenceRequirement;
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::workloads::types::discrepancy_evidence::value::{
    ContractSourcePath, SourceRevision, WitnessCase,
};
use sha2::{Digest as _, Sha256};

/// Whether the artifacts were refused as not matching their declared identity.
fn invalid(result: Result<(), SuiteError>) -> bool {
    matches!(result, Err(SuiteError::Invalid(_)))
}
fn toy_prior_control() -> (serde_json::Value, EvidenceRequirement) {
    let hash = |bytes: &[u8]| format!("{:x}", Sha256::digest(bytes));
    let worker = "a".repeat(64);
    let query = "select id from toy";
    let profile = "tool = 'toy'\n";
    let source = "x\n";
    let source_sha = hash(
        [b"module.py\0".as_slice(), source.as_bytes()]
            .concat()
            .as_slice(),
    );
    let finding = serde_json::json!({"rule":"toy", "file":"module.py", "start":0, "end":1});
    let receipt = serde_json::json!({
        "side":"candidate", "name":"positive", "complete":true, "error":null,
        "case_passed":true, "source_sha256":source_sha, "skipped":[],
        "diagnostics":[finding.clone()], "expected":[finding],
        "coverage":{"selected_rule_file_pairs":1,"evaluated_rule_file_pairs":1,
            "unknown_or_unread_rule_file_pairs":0,"skipped_rule_file_pairs":0}
    })
    .to_string()
        + "\n";
    let query_sha = hash(query.as_bytes());
    let profile_sha = hash(profile.as_bytes());
    let receipt_sha = hash(receipt.as_bytes());
    let execution = serde_json::json!({
        "schema":"prior-worker-execution/v1", "command":[format!("/sealed/{worker}"),
            "verify-cases", "toy", "/stage"], "stage_root":"/stage", "status":1,
        "worker_sha256":worker, "control_source_sha256":source_sha,
        "control_query_sha256":query_sha, "control_profile_sha256":profile_sha,
        "raw_receipt_sha256":receipt_sha, "witness_case":"positive"
    })
    .to_string();
    let evidence = serde_json::json!({
        "schema":"prior-worker-control/v1", "worker_sha256":worker,
        "control_source_sha256":source_sha, "control_query_sha256":query_sha,
        "control_profile_sha256":profile_sha, "witness_case":"positive",
        "control_sources":{"module.py":source}, "control_query":query,
        "control_profile":profile, "raw_receipt":receipt,
        "raw_receipt_sha256":receipt_sha, "execution_manifest":execution,
        "execution_manifest_sha256":hash(execution.as_bytes())
    });
    let requirement = EvidenceRequirement::PriorWorkerRun {
        worker_sha256: ArtifactDigest(worker),
        control_source_sha256: ArtifactDigest(source_sha),
        control_query_sha256: ArtifactDigest(query_sha),
        control_profile_sha256: ArtifactDigest(profile_sha),
        witness_case: WitnessCase("positive".into()),
    };
    (evidence, requirement)
}

fn prior_control_requires_exact_execution_identity_and_completed_positive_witness() {
    let (evidence, requirement) = toy_prior_control();
    let check = |evidence: &serde_json::Value, requirement: &EvidenceRequirement| {
        verify_requirement(&serde_json::to_vec(evidence).unwrap(), requirement)
    };
    check(&evidence, &requirement).expect("neutral complete control");
    for changed in [
        "worker_sha256",
        "control_query_sha256",
        "control_profile_sha256",
    ] {
        let mut requirement = requirement.clone();
        if let EvidenceRequirement::PriorWorkerRun {
            worker_sha256,
            control_query_sha256,
            control_profile_sha256,
            ..
        } = &mut requirement
        {
            *match changed {
                "worker_sha256" => worker_sha256,
                "control_query_sha256" => control_query_sha256,
                _ => control_profile_sha256,
            } = ArtifactDigest("b".repeat(64));
        }
        assert!(
            invalid(check(&evidence, &requirement)),
            "metadata-only {changed} mutation"
        );
    }
    let mut changed = evidence.clone();
    let mut execution: serde_json::Value =
        serde_json::from_str(changed["execution_manifest"].as_str().unwrap()).unwrap();
    execution["status"] = serde_json::json!(2);
    let execution = execution.to_string();
    changed["execution_manifest"] = serde_json::json!(execution);
    changed["execution_manifest_sha256"] =
        serde_json::json!(format!("{:x}", Sha256::digest(execution.as_bytes())));
    assert!(
        invalid(check(&changed, &requirement)),
        "failed prior execution"
    );

    let mut changed = evidence;
    let raw = "";
    let raw_sha = format!("{:x}", Sha256::digest(raw.as_bytes()));
    changed["raw_receipt"] = serde_json::json!(raw);
    changed["raw_receipt_sha256"] = serde_json::json!(raw_sha);
    let mut execution: serde_json::Value =
        serde_json::from_str(changed["execution_manifest"].as_str().unwrap()).unwrap();
    execution["raw_receipt_sha256"] = serde_json::json!(raw_sha);
    let execution = execution.to_string();
    changed["execution_manifest"] = serde_json::json!(execution);
    changed["execution_manifest_sha256"] =
        serde_json::json!(format!("{:x}", Sha256::digest(execution.as_bytes())));
    assert!(
        invalid(check(&changed, &requirement)),
        "one-sided control lacks candidate witness"
    );
}

fn public_contract_requires_pinned_source_and_declared_assertion() {
    let source = "/// Explicit provider selects requested source.\n";
    let source_sha = format!("{:x}", Sha256::digest(source.as_bytes()));
    let evidence = serde_json::json!({
        "schema":"public-tool-contract/v1", "source_revision":"c".repeat(40),
        "source_path":"crates/tool/src/api.rs", "source_sha256":source_sha,
        "contract_assertion":"Explicit provider selects requested source.",
        "source_text":source
    });
    let requirement = EvidenceRequirement::PublicToolContract {
        source_revision: SourceRevision("c".repeat(40)),
        source_path: ContractSourcePath("crates/tool/src/api.rs".into()),
    };
    let check = |value: &serde_json::Value| {
        verify_requirement(&serde_json::to_vec(value).unwrap(), &requirement)
    };
    check(&evidence).expect("neutral pinned source");
    let mut changed = evidence.clone();
    changed["source_path"] = serde_json::json!("crates/other.rs");
    assert!(invalid(check(&changed)));
    let mut changed = evidence;
    changed["contract_assertion"] = serde_json::json!("unpublished policy");
    assert!(invalid(check(&changed)));
}

type Scenario = fn();
type Input = Scenario;
type Value = ();
type Failure = std::convert::Infallible;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("prior_control_requires_exact_execution_identity_and_completed_positive_witness"),
        word: Word::Succeeds,
        input: || prior_control_requires_exact_execution_identity_and_completed_positive_witness as fn(),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("public_contract_requires_pinned_source_and_declared_assertion"),
        word: Word::Succeeds,
        input: || public_contract_requires_pinned_source_and_declared_assertion as fn(),
        expect: Expect::Value(()),
    },
];
fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario();
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
