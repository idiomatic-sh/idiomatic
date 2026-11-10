//! The original exact snapshot/source/evidence assertions through the actual public operation.
use super::interface::StageSuiteUseCase as _;
use crate::inbound::use_cases::prepare_cases::fixtures::workload_snapshot_preserves_exact_patch;
use crate::outbound::suite::{
    factory::local,
    interface::SuitePort as _,
    types::{ArtifactPath, ArtifactSet},
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use std::path::Path;

mod discrepancy;

/// The public operation under test, given the suite, the profile directory and the destination.
pub type Operate =
    fn(&dyn crate::outbound::suite::interface::SuitePort, &Path, &Path) -> Result<(), ReportsError>;
/// The seal of one staged tree.
fn seal(
    path: &std::path::Path,
) -> Result<
    ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest,
    ryo_benchmark_contracts::domain::reports::errors::ReportsError,
> {
    local().fingerprint(ArtifactSet(std::slice::from_ref(&path.to_path_buf())))
}

fn selected_snapshot(
    directory: &std::path::Path,
    destination: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(directory.join("fragments"))?;
    std::fs::write(
        directory.join("fragments/suppress.ryoql"),
        "fragment source",
    )?;
    std::fs::write(
        directory.join("benchmark.toml"),
        "tool='fixture'\nversion='1'\nsha256=['pin']\ndefaults=['E722','F704']\n[query-modules]\n'python/ruff/suppress'='fragments/suppress.ryoql'\n",
    )?;
    for rule in ["E722", "F704"] {
        let root = directory.join("rules").join(rule);
        std::fs::create_dir_all(&root)?;
        std::fs::write(root.join("query.ryoql"), rule)?;
        std::fs::write(root.join("cases.toml"), "cases = []\n")?;
    }
    let selected = vec![ryo::linter::RuleId::try_from("E722")?];
    let snapshot = destination.join("profile");
    crate::inbound::use_cases::stage_suite::use_case::StageSuiteUseCaseImpl::new(&local())
        .stage_suite(
            directory,
            &snapshot,
            &selected,
            ryo_benchmark_contracts::domain::workloads::types::StagePurpose::Certification,
        )?;
    let children = std::fs::read_dir(snapshot.join("rules"))?
        .map(|entry| entry.map(|entry| entry.file_name()))
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(children, [std::ffi::OsString::from("E722")]);
    let fingerprint = seal(&snapshot)?;
    assert_eq!(
        std::fs::read_to_string(snapshot.join("fragments/suppress.ryoql"))?,
        "fragment source"
    );
    std::fs::write(
        directory.join("fragments/suppress.ryoql"),
        "concurrent module edit",
    )?;
    assert_eq!(fingerprint, seal(&snapshot)?);
    std::fs::write(
        snapshot.join("fragments/suppress.ryoql"),
        "changed staged module",
    )?;
    assert_ne!(fingerprint, seal(&snapshot)?);
    let changed_module_fingerprint = seal(&snapshot)?;
    std::fs::write(directory.join("rules/F704/query.ryoql"), "concurrent work")?;
    assert_eq!(changed_module_fingerprint, seal(&snapshot)?);
    std::fs::write(
        snapshot.join("rules/E722/query.ryoql"),
        "changed selected rule",
    )?;
    assert_ne!(fingerprint, seal(&snapshot)?);
    Ok(())
}

fn supporting_queries_are_staged_but_never_receive_default_or_case_owner_credit()
-> Result<(), Box<dyn std::error::Error>> {
    let suite = local();
    let source = tempfile::tempdir()?;
    let destination = tempfile::tempdir()?;
    let root = source.path();
    std::fs::create_dir_all(root.join("rules/A001"))?;
    std::fs::create_dir_all(root.join("rules/B002"))?;
    std::fs::write(
        root.join("benchmark.toml"),
        "tool='fixture'\nversion='1'\nsha256=['pin']\ndefaults=['A001']\nsupporting-rules=['B002']\n",
    )?;
    std::fs::write(root.join("rules/A001/query.ryoql"), "main query")?;
    std::fs::write(root.join("rules/B002/query.ryoql"), "actual support query")?;
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
    )?;
    let tool = local().profile(ArtifactPath(root))?;
    let audited = crate::application::suite_auditing::service::audit(&local(), root, &tool)?;
    assert_eq!(
        audited.implemented,
        vec![ryo::linter::RuleId::try_from("A001")?]
    );
    assert!(audited.missing.is_empty());
    let snapshot = destination.path().join("profile");
    crate::inbound::use_cases::stage_suite::use_case::StageSuiteUseCaseImpl::new(&local())
        .stage_suite(
            root,
            &snapshot,
            &audited.implemented,
            ryo_benchmark_contracts::domain::workloads::types::StagePurpose::Certification,
        )?;
    assert_eq!(
        std::fs::read_to_string(snapshot.join("rules/B002/query.ryoql"))?,
        "actual support query"
    );
    let cases = crate::application::case_preparation::service::cases(
        &local(),
        &crate::outbound::context::factory::local(),
        &snapshot.join("rules"),
        &destination.path().join("scratch"),
    )?;
    assert_eq!(cases.len(), 2);
    assert!(cases.iter().all(|case| case.rule.as_ref() == "A001"));
    assert!(
        suite
            .case_claims(
                ArtifactPath(&snapshot),
                &suite.profile(ArtifactPath(&snapshot))?.defaults
            )?
            .is_empty()
    );
    // Support definitions do not acquire case ownership even if an unrelated file is present.
    std::fs::write(
        snapshot.join("rules/B002/cases.toml"),
        "not a case registration",
    )?;
    assert!(
        suite
            .case_claims(
                ArtifactPath(&snapshot),
                &suite.profile(ArtifactPath(&snapshot))?.defaults
            )?
            .is_empty()
    );
    // Missing owned case evidence remains an error, rather than being silently skipped.
    std::fs::remove_file(snapshot.join("rules/A001/cases.toml"))?;
    assert!(
        suite
            .case_claims(
                ArtifactPath(&snapshot),
                &suite.profile(ArtifactPath(&snapshot))?.defaults
            )
            .is_err()
    );
    std::fs::remove_file(root.join("rules/B002/query.ryoql"))?;
    assert!(crate::application::suite_auditing::service::audit(&local(), root, &tool).is_err());
    Ok(())
}
fn selected_snapshot_preserves_exact_sources() -> Result<(), Box<dyn std::error::Error>> {
    let source = tempfile::tempdir()?;
    let destination = tempfile::tempdir()?;
    selected_snapshot(source.path(), destination.path())
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
///
/// # Errors
/// The first failure to write a fixture source or to stage, seal or read it back.
///
/// # Panics
/// When a staged snapshot breaks its promise: a source edit reaches it, an edit of it keeps its seal, or a supporting
/// query gains credit.
pub fn exercise(scenario: Scenario) -> Result<(), Box<dyn std::error::Error>> {
    match scenario {
        Scenario::Subset => selected_snapshot_preserves_exact_sources(),
        Scenario::Workload => workload_snapshot_preserves_exact_patch(),
        Scenario::Supporting => {
            supporting_queries_are_staged_but_never_receive_default_or_case_owner_credit()
        }
        Scenario::Discrepancy => {
            discrepancy::discrepancy_artifacts_survive_staging_and_refuse_source_or_evidence_edits()
        }
    }
}
/// Stage a profile directory that does not exist: nothing is staged and the refusal names the missing profile.
///
/// # Errors
/// [`Refusal::MissingProfile`] when the profile is refused as missing, and [`Refusal::Unexpected`] for any other
/// failure.
///
/// # Panics
/// When the operation stages anything at the destination.
pub fn stage_missing(operate: Operate) -> Result<(), Refusal> {
    let root = tempfile::tempdir().map_err(|_| Refusal::Unexpected)?;
    let destination = root.path().join("snapshot");
    let result = operate(&local(), &root.path().join("missing"), &destination);
    assert_eq!(
        std::fs::symlink_metadata(&destination)
            .err()
            .map(|error| error.kind()),
        Some(std::io::ErrorKind::NotFound)
    );
    result.map_err(|error| match error {
        ReportsError::Io(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Refusal::MissingProfile
        }
        ReportsError::Invalid(_)
        | ReportsError::Io(_)
        | ReportsError::Json(_)
        | ReportsError::Toml(_) => Refusal::Unexpected,
    })
}
