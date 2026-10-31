//! A complete disposition inventory is distinct from verified subset coverage.
use crate::domain::{
    diagnostics::errors::Error,
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
pub fn certify(
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
    scope: crate::domain::suite::types::CoverageScope,
) -> Result<Certificate, Error> {
    let tool = suite.profile(directory)?;
    suite.verify_tool(reference_executable, &tool)?;
    let audit = suite.audit(directory, &tool)?;
    if scope == crate::domain::suite::types::CoverageScope::Accounted {
        require_accounted(&audit, &tool)?;
    }
    if audit.implemented.is_empty() {
        return Err(Error::Invalid("no authored default subset".into()));
    }
    let contestants = [
        reference_executable.canonicalize()?,
        candidate_executable.canonicalize()?,
    ];
    let paths = vec![
        directory.canonicalize()?,
        workload.canonicalize()?,
        selected_rules.canonicalize()?,
        contestants[0].clone(),
        contestants[1].clone(),
    ];
    let before = suite.fingerprint(&paths)?;
    let probe = crate::domain::suite::types::RunRequest {
        root: directory.to_owned(),
        files: vec!["__certificate_probe__.py".into()],
        cache: scratch.join("identity"),
    };
    let profiles = [reference.prepare(&probe)?, candidate.prepare(&probe)?];
    let cases = crate::application::cases::service::verify(
        suite,
        reference,
        candidate,
        execution,
        &directory.join("rules"),
        scratch,
    )?;
    let after = suite.fingerprint(&paths)?;
    if before != after {
        return Err(Error::Invalid(
            "certification inputs changed during cases".into(),
        ));
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
        cases,
    })
}
