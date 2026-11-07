//! Every evidence authority retains its exact artifact identity.
use super::value::{
    ContractSourcePath, DiscrepancyEvidence, DiscrepancyEvidenceKind, LanguageVersion, ParserTarget, Rationale,
    SourceRevision, WitnessCase,
};
use crate::domain::artifacts::types::digest::value::ArtifactDigest;
use crate::cases::{Case, CaseName, Expect, Word, run};
use std::borrow::Cow;

const CASES: &[Case<DiscrepancyEvidence, Cow<'static, str>, ()>] = &[
    Case {
        name: CaseName("language authority retains artifact"),
        word: Word::Succeeds,
        input: || DiscrepancyEvidence::Language {
            path: "evidence".into(),
            sha256: ArtifactDigest("pin".into()),
            kind: DiscrepancyEvidenceKind::LanguageSpec,
            language_version: LanguageVersion("3.11".into()),
            target_configuration: ParserTarget("python-target=3.11".into()),
            version_rationale: Rationale("exact version".into()),
        },
        expect: Expect::Value(Cow::Borrowed("pin")),
    },
    Case {
        name: CaseName("public contract retains artifact"),
        word: Word::Succeeds,
        input: || DiscrepancyEvidence::PublicToolContract {
            path: "evidence".into(),
            sha256: ArtifactDigest("pin".into()),
            source_revision: SourceRevision("revision".into()),
            source_path: ContractSourcePath("contract".into()),
            rationale: Rationale("policy".into()),
        },
        expect: Expect::Value(Cow::Borrowed("pin")),
    },
    Case {
        name: CaseName("prior worker authority retains artifact"),
        word: Word::Succeeds,
        input: || DiscrepancyEvidence::PriorWorkerRun {
            path: "evidence".into(),
            sha256: ArtifactDigest("pin".into()),
            worker_sha256: ArtifactDigest("worker".into()),
            control_source_sha256: ArtifactDigest("source".into()),
            control_query_sha256: ArtifactDigest("query".into()),
            control_profile_sha256: ArtifactDigest("profile".into()),
            witness_case: WitnessCase("completed".into()),
            rationale: Rationale("policy".into()),
        },
        expect: Expect::Value(Cow::Borrowed("pin")),
    },
];
fn subject(input: DiscrepancyEvidence) -> Result<Cow<'static, str>, ()> {
    let identity = input.artifact();
    assert_eq!(identity.path, std::path::Path::new("evidence"));
    Ok(identity.sha256.to_string().into())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
