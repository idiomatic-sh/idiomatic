//! A complete disposition inventory is distinct from verified subset coverage.
use crate::domain::{
    diagnostics::errors::Error,
    suite::{
        service::require_accounted,
        types::{Audit, Certificate},
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
    execution: &dyn ExecutionPort,
    directory: &Path,
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
        selected_rules.canonicalize()?,
        contestants[0].clone(),
        contestants[1].clone(),
    ];
    let before = suite.fingerprint(&paths)?;
    let cases = crate::outbound::suite::adapters::local::adapter::verify_cases(
        reference,
        execution,
        candidate_executable,
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
        cases,
    })
}
