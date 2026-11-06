//! A complete disposition inventory is distinct from verified subset coverage.
use crate::domain::suite::service::require_accounted;
use crate::domain::suite::types::audit::value::Audit;
use crate::domain::suite::types::certificate::value::Certificate;
use crate::outbound::{
    execution::interface::ExecutionPort, linter::interface::LinterPort, suite::interface::SuitePort,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
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
    inventory_store: &dyn crate::outbound::inventory::interface::InventoryPort,
    directory: &Path,
    workload: &Path,
    reference_executable: &Path,
    candidate_executable: &Path,
    selected_rules: &Path,
    scratch: &Path,
    scope: ryo_benchmark_contracts::domain::requests::types::BenchmarkCoverage,
    case_source: &ryo_benchmark_contracts::domain::requests::types::CertificationCases,
) -> Result<Certificate, Error> {
    let tool = suite.profile(crate::outbound::suite::types::ArtifactPath(directory))?;
    suite.verify_tool(
        crate::outbound::suite::types::ArtifactPath(reference_executable),
        &tool,
    )?;
    let audit = crate::application::suite_auditing::service::audit(suite, directory, &tool)?;
    if scope == ryo_benchmark_contracts::domain::requests::types::BenchmarkCoverage::Accounted {
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
    if let ryo_benchmark_contracts::domain::measurements::types::ExecutionProfile::Isolated(
        collector,
    ) = &execution_profile
    {
        paths.push(collector.executable.clone());
    }
    let prepared_cases = match case_source {
        ryo_benchmark_contracts::domain::requests::types::CertificationCases::Profile => {
            super::case_inventory::PreparedCases::Profile
        }
        ryo_benchmark_contracts::domain::requests::types::CertificationCases::Registered {
            registration,
            output,
        } => {
            let (registration, context_paths) = super::case_inventory::prepare(
                inventory_store,
                suite,
                registration,
                directory,
                &tool,
                selected_rules,
                reference_executable,
                candidate_executable,
            )?;
            paths.extend(context_paths);
            super::case_inventory::PreparedCases::Registered {
                registration,
                output: output.clone(),
            }
        }
    };
    let before = suite.fingerprint(crate::outbound::suite::types::ArtifactSet(&paths))?;
    let probe = ryo_benchmark_contracts::domain::requests::types::RunRequest {
        performance: Default::default(),
        selected_rules: None,
        rule_observations: None,
        rule_context: None,
        project_search_roots: None,
        given_project_inventory: Default::default(),
        staged_sources: None,
        file_metadata: None,
        context_manifest: None,
        root: directory.to_owned(),
        files: vec![ryo::linter::RelativePath(
            super::constants::PROFILE_PROBE.into(),
        )],
        state: scratch.join(super::constants::IDENTITY_STATE),
    };
    let profiles = [reference.prepare(&probe)?, candidate.prepare(&probe)?];
    let cases = match &prepared_cases {
        super::case_inventory::PreparedCases::Profile => {
            let rules = directory.join(super::constants::RULE_DIRECTORY);
            let prepared =
                crate::application::case_preparation::service::cases(suite, &rules, scratch)?;
            crate::application::run::service::verify_prepared_shard(
                suite,
                reference,
                candidate,
                execution,
                inventory_store,
                &rules,
                prepared,
                ryo_benchmark_contracts::domain::requests::types::CaseSelection::default(),
            )?
        }
        super::case_inventory::PreparedCases::Registered {
            registration,
            output,
        } => super::case_inventory::execute(registration, output, execution, inventory_store)?,
    };
    let after = suite.fingerprint(crate::outbound::suite::types::ArtifactSet(&paths))?;
    if before != after || execution.profile() != execution_profile {
        return Err(Error::Invalid(super::constants::CHANGED_EVIDENCE.into()));
    }
    if let ryo_benchmark_contracts::domain::requests::types::CertificationCases::Registered {
        output,
        ..
    } = case_source
    {
        paths.push(output.clone());
    }
    let sealed = suite.fingerprint(crate::outbound::suite::types::ArtifactSet(&paths))?;
    Ok(Certificate {
        coverage: Audit {
            implemented: audit.implemented,
            missing: Vec::new(),
        },
        paths,
        fingerprint: sealed,
        contestants,
        probe,
        profiles,
        execution: execution_profile,
        cases,
        case_source: case_source.clone(),
    })
}
