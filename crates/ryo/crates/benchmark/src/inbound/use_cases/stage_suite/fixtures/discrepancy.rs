//! A staged discrepancy case keeps its sealed source and evidence, and refuses an edit to either.
use crate::inbound::use_cases::stage_suite::interface::StageSuiteUseCase as _;
use crate::outbound::suite::{factory::local, interface::SuitePort as _, types::ArtifactPath};

/// Stage a discrepancy case, then edit its source and its evidence: each edit is refused.
pub(super) fn discrepancy_artifacts_survive_staging_and_refuse_source_or_evidence_edits()
-> Result<(), Box<dyn std::error::Error>> {
    use sha2::{Digest as _, Sha256};
    let parent = tempfile::tempdir()?;
    let source = parent.path().join("source");
    std::fs::create_dir_all(source.join("rules/T001"))?;
    std::fs::create_dir_all(source.join("evidence"))?;
    let sha = |bytes: &[u8]| format!("{:x}", Sha256::digest(bytes));
    let source_text = b"x\n";
    let source_sha = sha([b"case.txt\0".as_slice(), source_text].concat().as_slice());
    let authority = b"neutral independent meaning\n";
    let evidence_sha = sha(authority);
    std::fs::write(source.join("evidence/authority.txt"), authority)?;
    std::fs::write(
        source.join("benchmark.toml"),
        format!(
            "tool = 'toy'\nversion = '1'\nsha256 = ['{}']\ndefaults = ['T001']\nparser-configuration = 'toy-v1'\n",
            "a".repeat(64)
        ),
    )?;
    std::fs::write(
        source.join("rules/T001/query.ryoql"),
        "emit source subject file key(span);\n",
    )?;
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
    std::fs::write(source.join("rules/T001/cases.toml"), cases)?;
    let staged = parent.path().join("staged");
    let adapter = local();
    let tool = adapter.profile(ArtifactPath(&source))?;
    let audit = crate::application::suite_auditing::service::audit(&adapter, &source, &tool)?;
    crate::inbound::use_cases::stage_suite::use_case::StageSuiteUseCaseImpl::new(&adapter)
        .stage_suite(
            &source,
            &staged,
            &audit.implemented,
            ryo_benchmark_contracts::domain::workloads::types::StagePurpose::Certification,
        )?;
    let materialized = crate::application::case_preparation::service::cases(
        &adapter,
        &crate::outbound::context::factory::local(),
        &staged.join("rules"),
        &parent.path().join("cases"),
    )?;
    let wires = adapter.case_claims(
        ArtifactPath(&staged),
        &adapter.profile(ArtifactPath(&staged))?.defaults,
    )?;
    assert_eq!(wires.len(), 1);
    let wire = wires.first().ok_or("one authored discrepancy claim")?;
    let case = materialized
        .iter()
        .find(|case| case.name.to_string() == wire.name.0)
        .ok_or("the materialized discrepancy case")?;
    let verify_selected = || {
        verify_discrepancy_fixture(
            &adapter,
            &staged,
            &case.request.root,
            &case.request.files,
            wire,
        )
    };
    verify_selected()?;
    let file = case.request.root.join("case.txt");
    std::fs::write(&file, b"changed source")?;
    assert!(verify_selected().is_err(), "source mutation");
    std::fs::write(&file, source_text)?;
    std::fs::write(staged.join("evidence/authority.txt"), b"changed authority")?;
    assert!(verify_selected().is_err(), "evidence mutation");
    Ok(())
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
    use crate::outbound::suite::types::{
        ArtifactMetadata, ClaimedEvidence, ClaimedMetadata, EvidenceRequirement,
    };
    use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
    let digest =
        |text: &ryo_benchmark_contracts::domain::documents::types::text::value::Sha256Text| {
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
    adapter.verify_discrepancy_artifacts(crate::outbound::suite::types::CaseArtifacts {
        profile,
        root,
        files: &files,
        source_sha256: &source,
        file_metadata_files: &[],
        file_metadata: metadata
            .as_ref()
            .map_or(ArtifactMetadata::Undeclared, ArtifactMetadata::Declared),
        evidence: &evidence,
    })
}
