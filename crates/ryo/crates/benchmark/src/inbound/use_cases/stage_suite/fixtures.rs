//! The original exact snapshot/source/evidence assertions through the actual public operation.
use super::interface::StageSuiteUseCase as _;
use crate::inbound::use_cases::prepare_cases::fixtures::workload_snapshot_preserves_exact_patch;
use crate::outbound::suite::{
    factory::local,
    interface::SuitePort as _,
    types::{ArtifactPath, ArtifactSet},
};
/// The seal of one staged tree.
fn seal(path: &std::path::Path) -> ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest {
    local()
        .fingerprint(ArtifactSet(std::slice::from_ref(&path.to_path_buf())))
        .expect("seal")
}

fn selected_snapshot(directory: &std::path::Path, destination: &std::path::Path) {
    std::fs::create_dir_all(directory.join("fragments")).expect("fragment directory");
    std::fs::write(
        directory.join("fragments/suppress.ryoql"),
        "fragment source",
    )
    .expect("fragment source");
    std::fs::write(
        directory.join("benchmark.toml"),
        "tool='fixture'\nversion='1'\nsha256=['pin']\ndefaults=['E722','F704']\n[query-modules]\n'python/ruff/suppress'='fragments/suppress.ryoql'\n",
    )
    .expect("profile");
    for rule in ["E722", "F704"] {
        let root = directory.join("rules").join(rule);
        std::fs::create_dir_all(&root).expect("rule root");
        std::fs::write(root.join("query.ryoql"), rule).expect("query");
        std::fs::write(root.join("cases.toml"), "cases = []\n").expect("cases");
    }
    let selected = vec![ryo::linter::RuleId::try_from("E722").expect("rule identifier")];
    let snapshot = destination.join("profile");
    crate::inbound::use_cases::stage_suite::use_case::StageSuiteUseCaseImpl::new(&local())
        .stage_suite(
            directory,
            &snapshot,
            &selected,
            ryo_benchmark_contracts::domain::workloads::types::StagePurpose::Certification,
        )
        .expect("staged subset");
    let children: Vec<_> = std::fs::read_dir(snapshot.join("rules"))
        .expect("snapshot rules")
        .map(|entry| entry.expect("entry").file_name())
        .collect();
    assert_eq!(children, [std::ffi::OsString::from("E722")]);
    let fingerprint = seal(&snapshot);
    assert_eq!(
        std::fs::read_to_string(snapshot.join("fragments/suppress.ryoql")).expect("staged module"),
        "fragment source"
    );
    std::fs::write(
        directory.join("fragments/suppress.ryoql"),
        "concurrent module edit",
    )
    .expect("source module edit");
    assert_eq!(
        fingerprint,
        seal(&snapshot)
    );
    std::fs::write(
        snapshot.join("fragments/suppress.ryoql"),
        "changed staged module",
    )
    .expect("staged module edit");
    assert_ne!(
        fingerprint,
        seal(&snapshot)
    );
    let changed_module_fingerprint = seal(&snapshot);
    std::fs::write(directory.join("rules/F704/query.ryoql"), "concurrent work")
        .expect("unselected edit");
    assert_eq!(
        changed_module_fingerprint,
        seal(&snapshot)
    );
    std::fs::write(
        snapshot.join("rules/E722/query.ryoql"),
        "changed selected rule",
    )
    .expect("selected edit");
    assert_ne!(
        fingerprint,
        seal(&snapshot)
    );
}

fn supporting_queries_are_staged_but_never_receive_default_or_case_owner_credit() {
    let suite = local();
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    let root = source.path();
    std::fs::create_dir_all(root.join("rules/A001")).unwrap();
    std::fs::create_dir_all(root.join("rules/B002")).unwrap();
    std::fs::write(root.join("benchmark.toml"), "tool='fixture'\nversion='1'\nsha256=['pin']\ndefaults=['A001']\nsupporting-rules=['B002']\n").unwrap();
    std::fs::write(root.join("rules/A001/query.ryoql"), "main query").unwrap();
    std::fs::write(root.join("rules/B002/query.ryoql"), "actual support query").unwrap();
    std::fs::write(
        root.join("rules/A001/cases.toml"),
        r#"
[[cases]]
name = "positive"
files = { "input.txt" = "x" }
expected = [{ file = "input.txt", start = 0, end = 1 }]
[[cases]]
name = "negative"
files = { "input.txt" = "y" }
expected = []
"#,
    )
    .unwrap();
    let tool = local().profile(ArtifactPath(root)).unwrap();
    let audited =
        crate::application::suite_auditing::service::audit(&local(), root, &tool).unwrap();
    assert_eq!(audited.implemented, vec!["A001".try_into().unwrap()]);
    assert!(audited.missing.is_empty());
    let snapshot = destination.path().join("profile");
    crate::inbound::use_cases::stage_suite::use_case::StageSuiteUseCaseImpl::new(&local())
        .stage_suite(
            root,
            &snapshot,
            &audited.implemented,
            ryo_benchmark_contracts::domain::workloads::types::StagePurpose::Certification,
        )
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(snapshot.join("rules/B002/query.ryoql")).unwrap(),
        "actual support query"
    );
    let cases = crate::application::case_preparation::service::cases(
        &local(),
        &crate::outbound::context::factory::local(),
        &snapshot.join("rules"),
        &destination.path().join("scratch"),
    )
    .unwrap();
    assert_eq!(cases.len(), 2);
    assert!(cases.iter().all(|case| case.rule.as_ref() == "A001"));
    assert!(
        suite
            .case_claims(
                ArtifactPath(&snapshot),
                &suite.profile(ArtifactPath(&snapshot)).unwrap().defaults
            )
            .unwrap()
            .is_empty()
    );
    // Support definitions do not acquire case ownership even if an unrelated file is present.
    std::fs::write(
        snapshot.join("rules/B002/cases.toml"),
        "not a case registration",
    )
    .unwrap();
    assert!(
        suite
            .case_claims(
                ArtifactPath(&snapshot),
                &suite.profile(ArtifactPath(&snapshot)).unwrap().defaults
            )
            .unwrap()
            .is_empty()
    );
    // Missing owned case evidence remains an error, rather than being silently skipped.
    std::fs::remove_file(snapshot.join("rules/A001/cases.toml")).unwrap();
    assert!(
        suite
            .case_claims(
                ArtifactPath(&snapshot),
                &suite.profile(ArtifactPath(&snapshot)).unwrap().defaults
            )
            .is_err()
    );
    std::fs::remove_file(root.join("rules/B002/query.ryoql")).unwrap();
    assert!(crate::application::suite_auditing::service::audit(&local(), root, &tool).is_err());
}
fn discrepancy_artifacts_survive_staging_and_refuse_source_or_evidence_edits() {
    use sha2::{Digest as _, Sha256};
    let parent = tempfile::tempdir().expect("neutral fixture parent");
    let source = parent.path().join("source");
    std::fs::create_dir_all(source.join("rules/T001")).unwrap();
    std::fs::create_dir_all(source.join("evidence")).unwrap();
    let sha = |bytes: &[u8]| format!("{:x}", Sha256::digest(bytes));
    let source_text = b"x\n";
    let source_sha = sha([b"case.txt\0".as_slice(), source_text].concat().as_slice());
    let authority = b"neutral independent meaning\n";
    let evidence_sha = sha(authority);
    std::fs::write(source.join("evidence/authority.txt"), authority).unwrap();
    std::fs::write(source.join("benchmark.toml"), format!(
        "tool = 'toy'\nversion = '1'\nsha256 = ['{}']\ndefaults = ['T001']\nparser-configuration = 'toy-v1'\n",
        "a".repeat(64)
    )).unwrap();
    std::fs::write(
        source.join("rules/T001/query.ryoql"),
        "emit source subject file key(span);\n",
    )
    .unwrap();
    let pin = "a".repeat(64);
    let cases = format!(
        r#"[[cases]]
name = "source-identity"
comparison = "full"
files = {{ "case.txt" = "x\n" }}
expected = []
reference-discrepancy = {{ expected = [], identity = {{ tool = "toy", version = "1", observed-executable-sha256 = "{pin}", profile-sha256 = "{pin}", settings-sha256 = "{pin}", source-sha256 = "{source_sha}", reference-output-sha256 = "{pin}", intended-output-sha256 = "{pin}" }}, evidence = [{{ path = "evidence/authority.txt", sha256 = "{evidence_sha}", kind = "language-spec", language-version = "1", target-configuration = "toy-v1", version-rationale = "neutral control" }}], reason = "neutral source identity" }}

[[cases]]
name = "positive"
comparison = "full"
files = {{ "positive.txt" = "y\n" }}
expected = [{{ rule = "T001", file = "positive.txt", start = 0, end = 1 }}]
"#
    );
    std::fs::write(source.join("rules/T001/cases.toml"), cases).unwrap();
    let staged = parent.path().join("staged");
    let adapter = local();
    let tool = adapter
        .profile(ArtifactPath(&source))
        .expect("neutral profile");
    let audit = crate::application::suite_auditing::service::audit(&adapter, &source, &tool)
        .expect("neutral case audit");
    crate::inbound::use_cases::stage_suite::use_case::StageSuiteUseCaseImpl::new(&adapter)
        .stage_suite(
            &source,
            &staged,
            &audit.implemented,
            ryo_benchmark_contracts::domain::workloads::types::StagePurpose::Certification,
        )
        .expect("evidence staged");
    let materialized = crate::application::case_preparation::service::cases(
        &adapter,
        &crate::outbound::context::factory::local(),
        &staged.join("rules"),
        &parent.path().join("cases"),
    )
    .expect("typed case");
    let wires = adapter
        .case_claims(
            ArtifactPath(&staged),
            &adapter
                .profile(ArtifactPath(&staged))
                .expect("staged profile")
                .defaults,
        )
        .expect("authored evidence");
    assert_eq!(wires.len(), 1);
    let wire = &wires[0];
    let case = materialized
        .iter()
        .find(|case| case.name.to_string() == wire.name.0)
        .expect("materialized discrepancy case");
    let verify_selected = || {
        verify_discrepancy_fixture(
            &adapter,
            &staged,
            &case.request.root,
            &case.request.files,
            wire,
        )
    };
    verify_selected().expect("neutral source and evidence seal");
    let file = case.request.root.join("case.txt");
    std::fs::write(&file, b"changed source").unwrap();
    assert!(verify_selected().is_err(), "source mutation");
    std::fs::write(&file, source_text).unwrap();
    std::fs::write(staged.join("evidence/authority.txt"), b"changed authority").unwrap();
    assert!(verify_selected().is_err(), "evidence mutation");
}

/// Construct one explicit IO-boundary request from the staged fixture's external file identities.
fn verify_discrepancy_fixture(
    adapter: &impl crate::outbound::suite::interface::SuitePort,
    profile: &std::path::Path,
    root: &std::path::Path,
    input_files: &[ryo::linter::RelativePath],
    wire: &crate::outbound::suite::types::CaseClaim,
) -> Result<(), crate::outbound::suite::errors::SuiteError> {
    let files = input_files
        .iter()
        .map(|file| std::path::PathBuf::from(file.0.as_ref()))
        .collect::<Vec<_>>();
    use crate::outbound::suite::types::{ArtifactMetadata, ClaimedEvidence, ClaimedMetadata, EvidenceRequirement};
    use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
    let digest = |text: &ryo_benchmark_contracts::domain::documents::types::text::value::Sha256Text| {
        ArtifactDigest(text.0.clone())
    };
    let evidence = wire
        .discrepancy
        .evidence
        .iter()
        .map(|item| match item {
            ClaimedEvidence::LanguageSpec { path, sha256, .. }
            | ClaimedEvidence::IndependentLanguageRun { path, sha256, .. } => {
                crate::outbound::suite::types::EvidenceArtifact {
                    path: path.clone(),
                    sha256: digest(sha256),
                    requirement: EvidenceRequirement::Language,
                }
            }
            ClaimedEvidence::PublicToolContract {
                path,
                sha256,
                source_revision,
                source_path,
                ..
            } => crate::outbound::suite::types::EvidenceArtifact {
                path: path.clone(),
                sha256: digest(sha256),
                requirement: EvidenceRequirement::PublicToolContract {
                    source_revision: source_revision.clone(),
                    source_path: source_path.clone(),
                },
            },
            ClaimedEvidence::PriorWorkerRun {
                path,
                sha256,
                worker_sha256,
                control_source_sha256,
                control_query_sha256,
                control_profile_sha256,
                witness_case,
                ..
            } => crate::outbound::suite::types::EvidenceArtifact {
                path: path.clone(),
                sha256: digest(sha256),
                requirement: EvidenceRequirement::PriorWorkerRun {
                    worker_sha256: digest(worker_sha256),
                    control_source_sha256: digest(control_source_sha256),
                    control_query_sha256: digest(control_query_sha256),
                    control_profile_sha256: digest(control_profile_sha256),
                    witness_case: witness_case.clone(),
                },
            },
        })
        .collect::<Vec<_>>();
    let source = digest(&wire.discrepancy.identity.source_sha256);
    let metadata = match &wire.discrepancy.identity.file_metadata_sha256 {
        ClaimedMetadata::Declared(text) => Some(digest(text)),
        ClaimedMetadata::Undeclared => None,
    };
    adapter.verify_discrepancy_artifacts(
        crate::outbound::suite::types::CaseArtifacts {
            profile,
            root,
            files: &files,
            source_sha256: &source,
            file_metadata_files: &[],
            file_metadata: metadata
                .as_ref()
                .map_or(ArtifactMetadata::Undeclared, ArtifactMetadata::Declared),
            evidence: &evidence,
        },
    )
}

fn selected_snapshot_preserves_exact_sources() {
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    selected_snapshot(source.path(), destination.path());
}
#[derive(Clone, Copy, Debug)]
pub enum Scenario {
    Subset,
    Workload,
    Supporting,
    Discrepancy,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The profile directory does not exist.
    MissingProfile,
    /// Any other failure.
    Unexpected,
}
/// Exercise real source fixtures through the public staging operation.
pub fn exercise(scenario: Scenario) {
    match scenario {
        Scenario::Subset => selected_snapshot_preserves_exact_sources(),
        Scenario::Workload => workload_snapshot_preserves_exact_patch(),
        Scenario::Supporting => {
            supporting_queries_are_staged_but_never_receive_default_or_case_owner_credit()
        }
        Scenario::Discrepancy => {
            discrepancy_artifacts_survive_staging_and_refuse_source_or_evidence_edits()
        }
    }
}
