//! Project execution over deferred frontier sources and one publication scope.
use super::source_admission::ProjectSources;
use super::*;

pub(crate) struct ProjectRun<'a> {
    program: &'a Arc<PreparedProgram>,
    services: Arc<ProgramServices>,
    execution: &'a ProgramExecution,
    owner: Rc<RelationOwner>,
    required: RequiredPartitions,
    partials: ScopePartials<'a>,
    frontier_producer: RuleIndex,
    frontier_sources: Box<[crate::application::file_dag::frontiers::types::FrontierSource]>,
    ordinary: Vec<Option<crate::application::file_dag::frontiers::types::FrontierInput>>,
    refused: Vec<crate::domain::crossing::types::RefusedFrontier>,
    bound: Vec<Option<crate::application::file_dag::frontiers::types::BoundInput>>,
    cache: Vec<(
        crate::domain::executable_program::types::InputSource,
        AdmittedInput,
    )>,
    transport: ryo_language::domain::symbols::types::TransportPermit,
    context: super::super::context::RunContext<'a>,
    publication: &'a PublicationScope,
}
pub(crate) fn open_project<'a>(
    program: &'a Arc<PreparedProgram>,
    required: RequiredPartitions,
    partials: ScopePartials<'a>,
    mut frontiers: ProjectFrontiers<'a>,
    symbols: &mut ryo_language::domain::symbols::types::Symbols,
    arenas: &'a [ryo_values::domain::stored_facts::types::ArenaBytes],
    services: Arc<ProgramServices>,
) -> Result<ProjectRun<'a>, OperatingError> {
    if !matches!(partials, ScopePartials::Project { .. }) {
        return Err(OperatingError::WrongOperation);
    }
    frontiers
        .ordinary
        .validate_scope(program.program(), &required)?;
    crate::domain::spelling::service::check_name_spellings(
        frontiers.ordinary.rows.iter().flat_map(|input| -> Box<dyn Iterator<Item = ryo_relations::domain::relations::types::Name> + '_> {
            match input {
                crate::application::file_dag::frontiers::types::FrontierInput::Decoded { frontier, .. } => Box::new(frontier.table.cells.iter().filter_map(|value| match value { ryo_relations::domain::relations::types::Value::Name(name) => Some(*name), _ => None })),
                crate::application::file_dag::frontiers::types::FrontierInput::Compact { spelling, .. } => Box::new(spelling.iter().copied()),
            }
        }), symbols, arenas,
    )?;
    for input in &mut frontiers.ordinary.rows {
        if let crate::application::file_dag::frontiers::types::FrontierInput::Compact {
            spelling,
            ..
        } = input
        {
            spelling.clear();
        }
    }
    let scope = frontiers.publication;
    scope.validate_request(&services, &required)?;
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
        services: Arc::clone(&services),
        execution,
        owner,
        required,
        partials,
        frontier_producer: frontiers.ordinary.producer,
        frontier_sources: frontiers.ordinary.sources,
        ordinary: frontiers.ordinary.rows.into_iter().map(Some).collect(),
        refused: frontiers.ordinary.refused,
        bound: frontiers.bound_inputs.iter().cloned().map(Some).collect(),
        cache: Vec::new(),
        context: super::super::context::RunContext::project(
            &services,
            ryo_language::domain::symbols::service::transport_permit(symbols),
            arenas,
        ),
        transport,
        publication: scope,
    })
}
impl ProjectRun<'_> {
    pub(crate) fn rule(
        &mut self,
        rule: RuleIndex,
        demand: RuleDemand,
        symbols: &mut ryo_language::domain::symbols::types::Symbols,
    ) -> Result<ProjectRuleOutcome, OperatingError> {
        crate::application::file_dag::frontiers::types::validate_consumer_sources(
            self.program,
            self.frontier_producer,
            &self.frontier_sources,
            rule,
        )?;
        let mut context = self.context.borrow(symbols, self.program, rule)?;
        let context = &mut context;
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
            None,
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
                sources: &self.frontier_sources,
                ordinary: &mut self.ordinary,
                refused: &self.refused,
                bound: &mut self.bound,
                cache: &mut self.cache,
                transport: &self.transport,
                publication: self.publication,
            }),
        };
        let emit = match demand.emit {
            EmitDemand::Disabled => RootOutcome::NoQuery,
            EmitDemand::Enabled => match &metadata.emit_root {
                Ok(id)
                    if self.program.program().regions()[id.0].phase
                        != ryo_values::domain::plans::types::StepScope::CrossFile =>
                {
                    RootOutcome::Deferred {
                        phase: self.program.program().regions()[id.0].phase,
                    }
                }
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
                let result = if completion.observation_admission(&root.reads)
                    == crate::domain::completion::types::ObservationAdmission::Ready
                {
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
                            Ok(table) => ProjectObservation::Complete {
                                table,
                                publication: ObservedPublication::from_admitted(input),
                            },
                            Err(error) => ProjectObservation::Other(classify(error)),
                        },
                        Err(error) => ProjectObservation::Other(classify(error)),
                    },
                ));
            }
        }
        Ok(ProjectRuleOutcome {
            emit,
            observations,
            counts: self.execution.logical_counts(rule)?,
        })
    }
}
