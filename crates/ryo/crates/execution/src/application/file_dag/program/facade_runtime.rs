//! The application owns one execution and its admitted populations for each file
//! or project phase. Kernel code receives borrowed rows only after admission.
use super::{
    admission::{
        AdmittedInput, ApplicabilityProof, ApplicabilityReason, FileGeneration, Inputs, Partition,
        ProducerRoot, Readiness, RequiredPartitions,
    },
    facade::{
        ApplicabilityRefusal, CaptureContext, CaptureIntent, CapturedFrontier, DiagnosticDelivery,
        EmitDemand, FileRuleIntent, FrontierCapture, LogicalCounts, ObservationDemand,
        ObservedPublication, PreparedProgram, ProjectFrontiers, ProjectObservation,
        ProjectRuleOutcome, RootOutcome, RuleDemand, RuleOutcome,
    },
    runtime::{InvocationOutcome, KernelInvocation, ProgramExecution},
};
use ryo_relations::domain::relations::types::{FileId, Relation, Value};
use ryo_values::domain::{
    plan_parts::types::{Input, InputNumber},
    rows::types::Table,
    runs::types::Skipped,
    stored_facts::types::Bucket,
};
use std::{collections::HashMap, rc::Rc, sync::Arc};
use {
    crate::application::file_dag::relations::PublishedInput,
    crate::application::file_dag::relations::RelationOwner,
    crate::application::file_dag::scope::ScopePartials,
    crate::application::file_dag::scope::open_program_owner,
    crate::domain::completion::types::completion_policy::value::CompletionPolicy,
    crate::domain::coverage::types::Covered, crate::domain::coverage::types::RuleIndex,
    crate::domain::operating::errors::OperatingError,
    crate::domain::operating::types::OperatorContext, crate::domain::replay::service::scan,
    crate::domain::table_model::service::count_of,
    crate::domain::table_model::service::materialize, crate::domain::table_model::types::Rows,
    crate::outbound::derived::interface::DerivedPort, ryo_values::domain::rows::service::reset,
};

pub(crate) struct ProgramServices {
    request: Arc<super::request_identity::RequestIdentity>,
    kernels: Arc<dyn super::super::executor::PrimitiveExecutor + Send>,
    derived: Arc<dyn DerivedPort>,
}
impl ProgramServices {
    pub(crate) fn new(
        kernels: Arc<dyn super::super::executor::PrimitiveExecutor + Send>,
        derived: Arc<dyn DerivedPort>,
        corpus: &[ryo_values::domain::files::types::CorpusInput],
    ) -> Self {
        Self {
            kernels,
            derived,
            request: Arc::new(super::request_identity::RequestIdentity::new(corpus)),
        }
    }
    pub(super) fn request(&self) -> &Arc<super::request_identity::RequestIdentity> {
        &self.request
    }
    pub(super) fn kernels(&self) -> &dyn super::super::executor::PrimitiveExecutor {
        self.kernels.as_ref()
    }
    pub(super) fn derived(&self) -> &Arc<dyn DerivedPort> {
        &self.derived
    }
}

/// A named project phase and its bound inputs must use the very same owner.
pub(crate) struct PublicationScope {
    request: Arc<super::request_identity::RequestIdentity>,
    program: Arc<crate::domain::executable_program::types::ExecutableProgram>,
    owner: Rc<RelationOwner>,
    execution: std::cell::OnceCell<ProgramExecution>,
    symbols: std::cell::OnceCell<ryo_language::domain::symbols::types::TransportPermit>,
}
impl PublicationScope {
    pub(crate) fn finish(self) -> Result<(), OperatingError> {
        match self.execution.into_inner() {
            Some(execution) => execution.finish(),
            None => {
                self.owner.release_population();
                Ok(())
            }
        }
    }
    pub(super) fn program(
        &self,
    ) -> &Arc<crate::domain::executable_program::types::ExecutableProgram> {
        &self.program
    }
    pub(in crate::application::file_dag) fn owner(&self) -> &Rc<RelationOwner> {
        &self.owner
    }
    pub(super) fn validate_partitions(
        &self,
        required: &RequiredPartitions,
    ) -> Result<(), OperatingError> {
        if self.request.as_ref() == required.request.as_ref() {
            Ok(())
        } else {
            Err(OperatingError::NotRead)
        }
    }
    pub(super) fn validate_request(
        &self,
        services: &ProgramServices,
        required: &RequiredPartitions,
    ) -> Result<(), OperatingError> {
        if self.request.as_ref() != services.request.as_ref()
            || self.request.as_ref() != required.request.as_ref()
        {
            return Err(OperatingError::NotRead);
        }
        Ok(())
    }
    pub(super) fn validate_query(
        &self,
        symbols: &ryo_language::domain::symbols::types::QuerySymbols<'_>,
    ) -> Result<(), OperatingError> {
        let expected = self.symbols.get().ok_or(OperatingError::NotRead)?;
        if ryo_language::domain::symbols::service::query_has_transport_generation(symbols, expected)
        {
            Ok(())
        } else {
            Err(OperatingError::NotRead)
        }
    }
    fn claim_symbols(
        &self,
        symbols: &mut ryo_language::domain::symbols::types::Symbols,
    ) -> Result<ryo_language::domain::symbols::types::TransportPermit, OperatingError> {
        use ryo_language::domain::symbols::service::{same_transport_generation, transport_permit};
        let current = transport_permit(symbols);
        if let Some(expected) = self.symbols.get() {
            if !same_transport_generation(expected, &current) {
                return Err(OperatingError::NotRead);
            }
        } else {
            self.symbols
                .set(transport_permit(symbols))
                .map_err(|_| OperatingError::NotRead)?;
        }
        Ok(current)
    }
}
pub(crate) fn publication_owner(
    program: &Arc<PreparedProgram>,
    services: &ProgramServices,
) -> Result<PublicationScope, OperatingError> {
    Ok(PublicationScope {
        request: Arc::clone(&services.request),
        program: Arc::clone(program.program()),
        owner: open_program_owner(program.program(), Arc::clone(services.derived()), true),
        execution: std::cell::OnceCell::new(),
        symbols: std::cell::OnceCell::new(),
    })
}

/// The finite physical files named by this project request. No absent frame is
/// synthesized from a missing index, and a skipped file is never complete-empty.
pub(crate) fn required_partitions(
    program: &Arc<PreparedProgram>,
    rule: RuleIndex,
    files: &[ryo_values::domain::files::types::CorpusInput],
    covered: &[Covered],
    skipped: &[Skipped],
    services: &ProgramServices,
) -> Result<RequiredPartitions, OperatingError> {
    program.rule_metadata(rule)?;
    RequiredPartitions::from_request(&services.request, files, covered, skipped)
}

mod source_admission;
use source_admission::{SourceInputs, observation_table, table_of};

mod file_execution;
mod project_execution;
use file_execution::{FileRoot, classify, outcome_of};
pub(crate) use file_execution::{FileRun, open_file};
pub(crate) use project_execution::{ProjectRun, open_project};

#[cfg(test)]
mod compiled_parity;
