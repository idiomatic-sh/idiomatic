//! Certification owns the workflow over suite, linter and execution contracts.
use super::interface::CertifyUseCase;
use super::types::{PreparedCases, RegisteredContext};
use crate::domain::suite::{
    service::require_accounted,
    types::{audit::value::Audit, certificate::value::Certificate},
};
use crate::outbound::{
    execution::interface::ExecutionPort, linter::interface::LinterPort, suite::interface::SuitePort,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::sync::Arc;
/// Establish independent correctness evidence for the selected execution profile.
pub struct CertifyUseCaseImpl {
    /// Artifact validation and exact content fingerprinting boundary.
    suite: Arc<dyn SuitePort>,
    /// Case receipt recording boundary.
    receipts: Arc<dyn crate::outbound::receipts::interface::ReceiptsPort>,
    /// Sealed disk context validation and fingerprinting boundary.
    context: Arc<dyn crate::outbound::context::interface::ContextPort>,
    /// Pinned reference tool preparation and decoding boundary.
    reference: Arc<dyn LinterPort>,
    /// Candidate tool preparation and decoding boundary.
    candidate: Arc<dyn LinterPort>,
    /// Process launch and resource observation boundary.
    execution: Arc<dyn ExecutionPort>,
    inventory: Arc<dyn crate::outbound::inventory::interface::InventoryPort>,
}
impl CertifyUseCaseImpl {
    /// Bind the four independently replaceable boundaries.
    pub fn new(
        suite: Arc<dyn SuitePort>,
        receipts: Arc<dyn crate::outbound::receipts::interface::ReceiptsPort>,
        context: Arc<dyn crate::outbound::context::interface::ContextPort>,
        reference: Arc<dyn LinterPort>,
        candidate: Arc<dyn LinterPort>,
        execution: Arc<dyn ExecutionPort>,
        inventory: Arc<dyn crate::outbound::inventory::interface::InventoryPort>,
    ) -> Self {
        Self {
            suite,
            receipts,
            context,
            reference,
            candidate,
            execution,
            inventory,
        }
    }
}
impl CertifyUseCase for CertifyUseCaseImpl {
    fn certify(
        &self,
        directory: &std::path::Path,
        workload: &std::path::Path,
        reference_executable: &std::path::Path,
        candidate_executable: &std::path::Path,
        selected_rules: &std::path::Path,
        scratch: &std::path::Path,
        scope: ryo_benchmark_contracts::domain::requests::types::BenchmarkCoverage,
        cases: &ryo_benchmark_contracts::domain::requests::types::CertificationCases,
    ) -> Result<Certificate, Error> {
        let suite = self.suite.as_ref();
        let receipts = self.receipts.as_ref();
        let context = self.context.as_ref();
        let reference = self.reference.as_ref();
        let candidate = self.candidate.as_ref();
        let execution = self.execution.as_ref();
        let inventory_store = self.inventory.as_ref();
        let case_source = cases;
        let tool = suite.profile(crate::outbound::suite::types::ArtifactPath(directory))?;
        suite.verify_tool(
            crate::outbound::suite::types::ArtifactPath(reference_executable),
            &tool,
            &reference.version_line(&tool),
        )?;
        let audit = crate::application::suite_auditing::service::audit(suite, directory, &tool)?;
        if scope == ryo_benchmark_contracts::domain::requests::types::BenchmarkCoverage::Accounted {
            if let ryo_benchmark_contracts::domain::requests::types::CertificationCases::Registered {
                coverage:
                    ryo_benchmark_contracts::domain::requests::types::RegisteredCoverage::Selected {
                        ..
                    },
                ..
            } = case_source
            {
                return Err(Error::Invalid(
                    super::constants::SELECTED_COVERAGE_REQUIRES_INTERIM.into(),
                ));
            }
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
                PreparedCases::Profile
            }
            ryo_benchmark_contracts::domain::requests::types::CertificationCases::Registered {
                registration,
                output,
                coverage,
            } => {
                let RegisteredContext {
                    registration,
                    paths: context_paths,
                } = super::service::prepare(
                    inventory_store,
                    suite,
                    registration,
                    directory,
                    &tool,
                    selected_rules,
                    reference_executable,
                    candidate_executable,
                    coverage,
                )?;
                paths.extend(context_paths);
                PreparedCases::Registered {
                    registration,
                    output: output.clone(),
                    coverage: coverage.clone(),
                }
            }
        };
        let before = suite.fingerprint(crate::outbound::suite::types::ArtifactSet(&paths))?;
        let crate::application::certification_probe::types::ProbedProfiles { probe, profiles } =
            crate::application::certification_probe::service::probe(
                reference, candidate, directory, scratch,
            )?;
        let cases = match &prepared_cases {
            PreparedCases::Profile => {
                let rules = directory.join(super::constants::RULE_DIRECTORY);
                let prepared = crate::application::case_preparation::service::cases(
                    suite, context, &rules, scratch,
                )?;
                super::service::verify_prepared_shard(
                    suite,
                    receipts,
                    context,
                    reference,
                    candidate,
                    execution,
                    inventory_store,
                    &rules,
                    prepared,
                    ryo_benchmark_contracts::domain::requests::types::CaseSelection::default(),
                )?
            }
            PreparedCases::Registered {
                registration,
                output,
                coverage,
            } => crate::application::inventory_verification::service::certify_registered(
                registration,
                output,
                suite,
                receipts,
                execution,
                inventory_store,
                coverage,
            )?,
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
}
impl std::fmt::Debug for CertifyUseCaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
