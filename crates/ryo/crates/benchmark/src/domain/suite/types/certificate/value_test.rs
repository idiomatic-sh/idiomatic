//! Proves one value's behaviour.
use super::value::Certificate;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::suite::types::audit::value::Audit;
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
use ryo_benchmark_contracts::domain::requests::types::{
    CohortSelection, ContextSeal, FileMetadataDeclaration, InputDeclaration,
    RuleContextPreparation, RunRequest, SearchRoots, StagedSources,
};
const CASES: &[Case<(), (bool, usize), ()>] = &[Case {
    name: CaseName("evidence exposes only its content identity and verified count"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value((true, 2)),
}];
#[expect(
    clippy::unnecessary_wraps,
    reason = "the shared case runner compares subjects through a uniform Result contract"
)]
fn subject(_: ()) -> Result<(bool, usize), ()> {
    let request = RunRequest {
        performance: Default::default(),
        selected_rules: CohortSelection::Frozen,
        rule_observations: InputDeclaration::Undeclared,
        rule_context: RuleContextPreparation::Unconfigured,
        project_search_roots: SearchRoots::Undeclared,
        given_project_inventory: Default::default(),
        staged_sources: StagedSources::Unlisted,
        file_metadata: FileMetadataDeclaration::Undeclared,
        context_manifest: ContextSeal::Unsealed,
        root: ".".into(),
        files: vec![ryo::linter::RelativePath("file".into())],
        state: "cache".into(),
    };
    let profile = Invocation {
        executable: "fixture".into(),
        arguments: Vec::new(),
        environment: std::collections::BTreeMap::new(),
        directory: ".".into(),
    };
    let certificate = Certificate {
        case_source: ryo_benchmark_contracts::domain::requests::types::CertificationCases::Profile,
        coverage: Audit {
            implemented: vec![ryo::linter::RuleId::try_from("A").unwrap()],
            missing: vec![],
        },
        paths: vec![],
        fingerprint:
            ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest(
                "sealed".into(),
            ),
        contestants: ["first".into(), "second".into()],
        probe: request,
        profiles: [profile.clone(), profile],
        execution: ryo_benchmark_contracts::domain::measurements::types::ExecutionProfile::Wall,
        cases: ryo_benchmark_contracts::domain::workloads::types::case_count::value::CaseCount(2),
    };
    Ok((
        certificate.fingerprint().0 == "sealed",
        certificate.cases().0,
    ))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
