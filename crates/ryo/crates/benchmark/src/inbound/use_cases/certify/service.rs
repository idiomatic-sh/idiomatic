//! A complete disposition inventory is distinct from verified subset coverage.
use crate::domain::{
    diagnostics::errors::DiagnosticsError as Error,
    suite::{
        service::require_accounted,
        types::{audit::value::Audit, certificate::value::Certificate},
    },
};
use crate::outbound::{
    execution::interface::ExecutionPort, linter::interface::LinterPort, suite::interface::SuitePort,
};
use std::path::Path;

/// Execute independent cases, then seal the unchanged artifacts and both executable bytes.
#[expect(
    clippy::too_many_arguments,
    reason = "this orchestration forwards explicit sealed workload identities through four independently injected ports"
)]
pub(super) fn certify(
    suite: &dyn SuitePort,
    reference: &dyn LinterPort,
    candidate: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    directory: &Path,
    workload: &Path,
    reference_executable: &Path,
    candidate_executable: &Path,
    selected_rules: &Path,
    scratch: &Path,
    scope: crate::domain::suite::types::BenchmarkCoverage,
) -> Result<Certificate, Error> {
    let tool = suite.profile(crate::outbound::suite::types::ArtifactPath(directory))?;
    suite.verify_tool(
        crate::outbound::suite::types::ArtifactPath(reference_executable),
        &tool,
    )?;
    let audit = suite.audit(
        crate::outbound::suite::types::ArtifactPath(directory),
        &tool,
    )?;
    if scope == crate::domain::suite::types::BenchmarkCoverage::Accounted {
        require_accounted(&audit, &tool)?;
    }
    if audit.implemented.is_empty() {
        return Err(Error::Invalid(
            crate::domain::suite::constants::EMPTY_SUBSET.into(),
        ));
    }
    let contestants = [
        reference_executable.to_owned(),
        candidate_executable.to_owned(),
    ];
    let execution_profile = execution.profile();
    let mut paths = vec![
        directory.to_owned(),
        workload.to_owned(),
        selected_rules.to_owned(),
        contestants[0].clone(),
        contestants[1].clone(),
    ];
    if let crate::domain::measurements::types::ExecutionProfile::Isolated(collector) =
        &execution_profile
    {
        paths.push(collector.executable.clone());
    }
    let before = suite.fingerprint(crate::outbound::suite::types::ArtifactSet(&paths))?;
    let probe = crate::domain::suite::types::RunRequest {
        performance: Default::default(),
        selected_rules: None,
        rule_observations: None,
        rule_context: None,
        file_metadata: None,
        context_manifest: None,
        root: directory.to_owned(),
        files: vec![ryo::linter::RelativePath(
            super::constants::PROFILE_PROBE.into(),
        )],
        state: scratch.join(super::constants::IDENTITY_STATE),
    };
    let profiles = [reference.prepare(&probe)?, candidate.prepare(&probe)?];
    let cases = crate::application::run::service::verify(
        suite,
        reference,
        candidate,
        execution,
        &directory.join(super::constants::RULE_DIRECTORY),
        scratch,
    )?;
    let after = suite.fingerprint(crate::outbound::suite::types::ArtifactSet(&paths))?;
    if before != after || execution.profile() != execution_profile {
        return Err(Error::Invalid(super::constants::CHANGED_EVIDENCE.into()));
    }
    Ok(Certificate {
        coverage: Audit {
            implemented: audit.implemented,
            missing: Vec::new(),
        },
        paths,
        fingerprint: after,
        contestants,
        probe,
        profiles,
        execution: execution_profile,
        cases,
    })
}
