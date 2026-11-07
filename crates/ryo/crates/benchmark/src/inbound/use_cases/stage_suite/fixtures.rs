//! The original exact snapshot/source/evidence assertions through the actual public operation.
use super::interface::StageSuiteUseCase as _;
use crate::outbound::suite::{
    factory::local,
    interface::SuitePort as _,
    types::{ArtifactPath, ArtifactSet, CaseReceipts},
};
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
    crate::inbound::use_cases::stage_suite::use_case::StageSuiteUseCaseImpl::new(&local(CaseReceipts::Unrecorded))
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
    let fingerprint = local(CaseReceipts::Unrecorded)
        .fingerprint(ArtifactSet(std::slice::from_ref(&snapshot)))
        .expect("seal");
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
        local(CaseReceipts::Unrecorded)
            .fingerprint(ArtifactSet(std::slice::from_ref(&snapshot)))
            .expect("unchanged staged module seal")
    );
    std::fs::write(
        snapshot.join("fragments/suppress.ryoql"),
        "changed staged module",
    )
    .expect("staged module edit");
    assert_ne!(
        fingerprint,
        local(CaseReceipts::Unrecorded)
            .fingerprint(ArtifactSet(std::slice::from_ref(&snapshot)))
            .expect("changed staged module seal")
    );
    let changed_module_fingerprint = local(CaseReceipts::Unrecorded)
        .fingerprint(ArtifactSet(std::slice::from_ref(&snapshot)))
        .expect("seal after staged module edit");
    std::fs::write(directory.join("rules/F704/query.ryoql"), "concurrent work")
        .expect("unselected edit");
    assert_eq!(
        changed_module_fingerprint,
        local(CaseReceipts::Unrecorded)
            .fingerprint(ArtifactSet(std::slice::from_ref(&snapshot)))
            .expect("unchanged seal")
    );
    std::fs::write(
        snapshot.join("rules/E722/query.ryoql"),
        "changed selected rule",
    )
    .expect("selected edit");
    assert_ne!(
        fingerprint,
        local(CaseReceipts::Unrecorded)
            .fingerprint(ArtifactSet(&[snapshot]))
            .expect("changed seal")
    );
}

fn workload_snapshot(directory: &std::path::Path, destination: &std::path::Path) {
    let shared = directory.join("shared");
    std::fs::create_dir_all(shared.join("patches")).expect("patch directory");
    std::fs::write(
        shared.join("benchmark.toml"),
        r#"language = "fixture"
[[projects]]
name = "project"
repository = "https://example.invalid/project.git"
revision = "revision"
patch = "patches/project.patch"
"#,
    )
    .expect("shared manifest");
    std::fs::write(shared.join("patches/project.patch"), "original patch").expect("patch");
    let frozen = destination.join("workload");
    local(CaseReceipts::Unrecorded)
        .stage_projects(ArtifactPath(&shared), ArtifactPath(&frozen))
        .expect("freeze workload");
    let seal = local(CaseReceipts::Unrecorded)
        .fingerprint(ArtifactSet(std::slice::from_ref(&frozen)))
        .expect("workload seal");
    std::fs::write(shared.join("patches/project.patch"), "concurrent edit").expect("source edit");
    assert_eq!(
        std::fs::read_to_string(frozen.join("patches/project.patch")).unwrap(),
        "original patch"
    );
    assert_eq!(
        seal,
        local(CaseReceipts::Unrecorded)
            .fingerprint(ArtifactSet(std::slice::from_ref(&frozen)))
            .unwrap()
    );
    std::fs::write(frozen.join("patches/project.patch"), "tampered snapshot")
        .expect("snapshot edit");
    assert_ne!(seal, local(CaseReceipts::Unrecorded).fingerprint(ArtifactSet(&[frozen])).unwrap());
}

fn supporting_queries_are_staged_but_never_receive_default_or_case_owner_credit() {
    let suite = local(CaseReceipts::Unrecorded);
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
    let tool = local(CaseReceipts::Unrecorded).profile(ArtifactPath(root)).unwrap();
    let audited =
        crate::application::suite_auditing::service::audit(&local(CaseReceipts::Unrecorded), root, &tool).unwrap();
    assert_eq!(audited.implemented, vec!["A001".try_into().unwrap()]);
    assert!(audited.missing.is_empty());
    let snapshot = destination.path().join("profile");
    crate::inbound::use_cases::stage_suite::use_case::StageSuiteUseCaseImpl::new(&local(CaseReceipts::Unrecorded))
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
        &local(CaseReceipts::Unrecorded),
        &snapshot.join("rules"),
        &destination.path().join("scratch"),
    )
    .unwrap();
    assert_eq!(cases.len(), 2);
    assert!(cases.iter().all(|case| case.rule.as_ref() == "A001"));
    assert!(
        suite
            .case_reference_wires(
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
            .case_reference_wires(
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
            .case_reference_wires(
                ArtifactPath(&snapshot),
                &suite.profile(ArtifactPath(&snapshot)).unwrap().defaults
            )
            .is_err()
    );
    std::fs::remove_file(root.join("rules/B002/query.ryoql")).unwrap();
    assert!(crate::application::suite_auditing::service::audit(&local(CaseReceipts::Unrecorded), root, &tool).is_err());
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
    let adapter = local(CaseReceipts::Unrecorded);
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
        &staged.join("rules"),
        &parent.path().join("cases"),
    )
    .expect("typed case");
    let wires = adapter
        .case_reference_wires(
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
        .find(|case| case.name.to_string() == wire.name)
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
    wire: &crate::outbound::suite::types::discrepancy::CaseReferenceWire,
) -> Result<(), crate::outbound::suite::types::discrepancy::ArtifactError> {
    let files = input_files
        .iter()
        .map(|file| std::path::PathBuf::from(file.0.as_ref()))
        .collect::<Vec<_>>();
    let evidence = wire
        .discrepancy
        .evidence
        .iter()
        .map(
            |item| crate::outbound::suite::types::discrepancy::EvidenceArtifact {
                path: item.artifact().0.to_path_buf(),
                sha256: item.artifact().1.to_owned(),
                requirement: item.requirement(),
            },
        )
        .collect::<Vec<_>>();
    adapter.verify_discrepancy_artifacts(
        crate::outbound::suite::types::discrepancy::CaseArtifacts {
            profile,
            root,
            files: &files,
            source_sha256: &wire.discrepancy.identity.source_sha256,
            file_metadata_files: &[],
            file_metadata_sha256: wire.discrepancy.identity.file_metadata_sha256.as_deref(),
            evidence: &evidence,
        },
    )
}

fn selected_snapshot_preserves_exact_sources() {
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    selected_snapshot(source.path(), destination.path());
}
fn workload_snapshot_preserves_exact_patch() {
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    workload_snapshot(source.path(), destination.path());
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
