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
use benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::sync::Arc;
/// Establish independent correctness evidence for the selected execution profile.
pub struct CertifyUseCaseImpl {
    /// Artifact validation and exact content fingerprinting boundary.
    suite: Arc<dyn SuitePort>,
    /// The host's file-permission facts.
    host: Arc<dyn crate::outbound::host::interface::HostPort>,
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
    /// Runs the registered rules' verification.
    scheduler: Arc<dyn crate::outbound::scheduler::interface::SchedulerPort>,
}
impl CertifyUseCaseImpl {
    /// Bind the independently replaceable boundaries.
    #[expect(
        clippy::too_many_arguments,
        reason = "each boundary is an independently injected port of the certification"
    )]
    pub fn new(
        suite: Arc<dyn SuitePort>,
        host: Arc<dyn crate::outbound::host::interface::HostPort>,
        receipts: Arc<dyn crate::outbound::receipts::interface::ReceiptsPort>,
        context: Arc<dyn crate::outbound::context::interface::ContextPort>,
        reference: Arc<dyn LinterPort>,
        candidate: Arc<dyn LinterPort>,
        execution: Arc<dyn ExecutionPort>,
        inventory: Arc<dyn crate::outbound::inventory::interface::InventoryPort>,
        scheduler: Arc<dyn crate::outbound::scheduler::interface::SchedulerPort>,
    ) -> Self {
        Self {
            suite,
            host,
            receipts,
            context,
            reference,
            candidate,
            execution,
            inventory,
            scheduler,
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
        scope: benchmark_contracts::domain::requests::types::BenchmarkCoverage,
        cases: &benchmark_contracts::domain::requests::types::CertificationCases,
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
        if scope == benchmark_contracts::domain::requests::types::BenchmarkCoverage::Accounted {
            if let benchmark_contracts::domain::requests::types::CertificationCases::Registered {
                coverage:
                    benchmark_contracts::domain::requests::types::RegisteredCoverage::Selected {
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
        let execution_profile = execution.profile();
        let mut paths = vec![
            directory.to_owned(),
            workload.to_owned(),
            selected_rules.to_owned(),
            reference_executable.to_owned(),
            candidate_executable.to_owned(),
        ];
        if let benchmark_contracts::domain::measurements::types::ExecutionProfile::Isolated(
            collector,
        ) = &execution_profile
        {
            paths.push(collector.executable.clone());
        }
        let prepared_cases = match case_source {
            benchmark_contracts::domain::requests::types::CertificationCases::Profile => {
                PreparedCases::Profile
            }
            benchmark_contracts::domain::requests::types::CertificationCases::Registered {
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
        let sealed = suite.fingerprint(crate::outbound::suite::types::ArtifactSet(&paths))?;
        let crate::application::certification_probe::types::ProbedProfiles { probe, profiles } =
            crate::application::certification_probe::service::probe(
                reference, candidate, directory, scratch,
            )?;
        if suite.fingerprint(crate::outbound::suite::types::ArtifactSet(&paths))? != sealed {
            return Err(Error::Invalid(
                super::constants::PROBE_CHANGED_INPUTS.into(),
            ));
        }
        // The program each probe's command launches is sealed with the rest: a launcher beside the certified binary
        // (clippy's cargo-clippy) must not change either.
        for profile in &profiles {
            if !paths.contains(&profile.executable) {
                paths.push(profile.executable.clone());
            }
        }
        let before = suite.fingerprint(crate::outbound::suite::types::ArtifactSet(&paths))?;
        let cases = match &prepared_cases {
            PreparedCases::Profile => {
                let rules = directory.join(super::constants::RULE_DIRECTORY);
                let prepared = crate::application::case_preparation::service::cases(
                    suite, context, &rules, scratch,
                )?;
                super::service::verify_prepared_shard(
                    suite,
                    self.host.as_ref(),
                    receipts,
                    context,
                    reference,
                    candidate,
                    execution,
                    inventory_store,
                    &rules,
                    prepared,
                    benchmark_contracts::domain::requests::types::CaseSelection::default(),
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
                self.scheduler.as_ref(),
            )?,
        };
        let after = suite.fingerprint(crate::outbound::suite::types::ArtifactSet(&paths))?;
        if before != after || execution.profile() != execution_profile {
            return Err(Error::Invalid(super::constants::CHANGED_EVIDENCE.into()));
        }
        if let benchmark_contracts::domain::requests::types::CertificationCases::Registered {
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
