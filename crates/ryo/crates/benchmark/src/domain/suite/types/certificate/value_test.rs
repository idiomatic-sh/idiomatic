//! Proves one value's behaviour.
use super::value::Certificate;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::suite::types::{RunRequest, audit::value::Audit};
use crate::outbound::linter::{fixtures::fake::LinterFake, interface::LinterPort as _};
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
        selected_rules: None,
        rule_observations: None,
        rule_context: None,
        project_search_roots: None,
        given_project_inventory: Default::default(),
        staged_sources: None,
        file_metadata: None,
        context_manifest: None,
        root: ".".into(),
        files: vec![ryo::linter::RelativePath("file".into())],
        state: "cache".into(),
    };
    let profile = LinterFake::default().prepare(&request).unwrap();
    let certificate = Certificate {
        coverage: Audit {
            implemented: vec![ryo::linter::RuleId::try_from("A").unwrap()],
            missing: vec![],
        },
        paths: vec![],
        fingerprint: crate::domain::suite::types::ArtifactDigest("sealed".into()),
        contestants: ["first".into(), "second".into()],
        probe: request,
        profiles: [profile.clone(), profile],
        execution: crate::domain::measurements::types::ExecutionProfile::Wall,
        cases: crate::domain::suite::types::CaseCount(2),
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
