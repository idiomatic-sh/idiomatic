//! Project execution over deferred frontier sources and one publication scope.
use super::source_admission::ProjectSources;
use super::*;

pub(crate) struct ProjectRun<'a, 'service> {
    program: &'a Arc<PreparedProgram>,
    services: &'a ProgramServices<'service>,
    execution: &'a ProgramExecution,
    owner: Rc<RelationOwner>,
    required: RequiredPartitions,
    partials: ScopePartials<'a>,
    ordinary: Vec<Option<crate::application::cross_file::types::FrontierInput>>,
    refused: Vec<crate::domain::crossing::types::RefusedFrontier>,
    bound: Vec<Option<crate::domain::crossing::types::BoundInputWords>>,
    cache: Vec<(
        crate::domain::executable_program::types::InputSource,
        AdmittedInput,
    )>,
    transport: ryo_language::domain::symbols::types::TransportPermit,
}
pub(crate) fn open_project<'a, 'service>(
    program: &'a Arc<PreparedProgram>,
    required: RequiredPartitions,
    partials: ScopePartials<'a>,
    frontiers: ProjectFrontiers<'a>,
    symbols: &mut ryo_language::domain::symbols::types::Symbols,
    services: &'a ProgramServices<'service>,
) -> Result<ProjectRun<'a, 'service>, OperatingError> {
    if !matches!(partials, ScopePartials::Project { .. }) {
        return Err(OperatingError::WrongOperation);
    }
    let scope = frontiers.publication;
    let transport = scope.claim_symbols(symbols)?;
    let owner = Rc::clone(scope.owner());
    if let Some(existing) = scope.execution.get() {
        if !Arc::ptr_eq(&existing.program, program.program())
            || !existing.project_partition_matches(&required)
        {
            return Err(OperatingError::WrongOperation);
        }
    } else {
        scope
            .execution
            .set(ProgramExecution::open(
                Arc::clone(program.program()),
                Partition::Project(required.clone()),
                Rc::clone(&owner),
            ))
            .map_err(|_| OperatingError::WrongOperation)?;
    }
    let execution = scope.execution.get().ok_or(OperatingError::NotRun)?;
    Ok(ProjectRun {
        program,
        services,
        execution,
        owner,
        required,
        partials,
        ordinary: frontiers.ordinary.rows.into_iter().map(Some).collect(),
        refused: frontiers.ordinary.refused,
        bound: frontiers.bound_inputs.iter().cloned().map(Some).collect(),
        cache: Vec::new(),
        transport,
    })
}
impl ProjectRun<'_, '_> {
    pub(crate) fn rule(
        &mut self,
        rule: RuleIndex,
        demand: RuleDemand,
        context: &mut OperatorContext<'_>,
    ) -> Result<RuleOutcome, OperatingError> {
        if !ryo_language::domain::symbols::service::query_has_transport_generation(
            &context.symbols,
            &self.transport,
        ) {
            return Err(OperatingError::NotRead);
        }
        let metadata = self.program.rule_metadata(rule)?;
        let completion = match &self.partials {
            ScopePartials::Project {
                evaluation,
                publication,
            } => CompletionPolicy::project_inputs(rule, evaluation, publication),
            ScopePartials::File { .. } => return Err(OperatingError::WrongOperation),
        };
        let unavailable = |_relation| Err(OperatingError::NotRead);
        let kernels = super::super::kernel_session::KernelSession::new(
            self.services.kernels(),
            self.program,
            Rc::clone(&self.owner),
            &unavailable,
        );
        let partition = Partition::Project(self.required.clone());
        let mut absent = HashMap::new();
        let mut sources = SourceInputs {
            execution: self.execution,
            completion: completion.clone(),
            owner: Rc::clone(&self.owner),
            read: &unavailable,
            kernels: &kernels,
            observations: &mut absent,
            file_outputs: None,
            program: self.program,
            rule,
            project: Some(ProjectSources {
                program: self.program,
                rule,
                ordinary: &mut self.ordinary,
                refused: &self.refused,
                bound: &mut self.bound,
                cache: &mut self.cache,
                transport: &self.transport,
            }),
        };
        let emit = match demand.emit {
            EmitDemand::Disabled => RootOutcome::NoQuery,
            EmitDemand::Enabled => match &metadata.emit_root {
                Ok(id) => outcome_of(
                    self.execution.invoke(
                        *id,
                        &mut sources,
                        &kernels,
                        &completion,
                        context,
                        Delivery::Retain,
                    ),
                    &self.owner,
                    &unavailable,
                ),
                Err(error) => RootOutcome::Failed(error.clone()),
            },
        };
        let mut observations = Vec::new();
        if matches!(demand.observations, ObservationDemand::Required) {
            for (index, root) in metadata.observations.iter().enumerate() {
                if root.phase != ryo_values::domain::plans::types::StepScope::CrossFile {
                    continue;
                }
                let result = if completion.observation_ready(&root.reads) {
                    root.source
                        .clone()
                        .and_then(|source| sources.admit(source, &partition, context))
                } else {
                    Err(OperatingError::ObservationIncomplete {
                        reason: "physical source facts are incomplete".into(),
                    })
                };
                observations.push((
                    ryo_values::domain::observations::types::ObservationIndex(
                        u32::try_from(index).map_err(|_| OperatingError::ShapeMismatch)?,
                    ),
                    match result {
                        Ok(input) => match observation_table(&self.owner, &input, &unavailable) {
                            Ok(table) => RootOutcome::Complete(table),
                            Err(error) => classify(error),
                        },
                        Err(error) => classify(error),
                    },
                ));
            }
        }
        Ok(RuleOutcome {
            emit,
            observations,
            counts: self.execution.logical_counts(rule)?,
        })
    }
}
