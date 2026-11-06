//! Project execution over deferred frontier sources and one publication scope.
use super::super::project_population::types::{
    BoundKey, ProjectFrontierPayload, ProjectFrontierPopulation, ProjectInvocationKey,
    ProjectRefusal, ProjectSourceMatch,
};
use super::source_admission::ProjectSources;
use super::*;
use crate::application::file_dag::{derived::owner::PopulationId, frontiers::types::FrontierInput};
use crate::domain::executable_program::types::InputSource;
impl ProjectFrontierPopulation<AvailableInput> {
    pub(super) fn source(&self) -> &Result<InputSource, OperatingError> {
        &self.source
    }
    pub(super) fn supply(
        &self,
        owner: &Rc<RelationOwner>,
        partition: &Partition,
    ) -> Result<AvailableInput, OperatingError> {
        self.admitted
            .get_or_init(|| {
                let source = self.source.clone()?;
                let publication = match &self.payload {
                    ProjectFrontierPayload::Flat(table) => {
                        owner.source_shared_computed(Arc::clone(table))
                    }
                    ProjectFrontierPayload::Compact(table) => {
                        owner.source_compact(Arc::clone(table))
                    }
                }?;
                let unavailable = |_| Err(OperatingError::NotRead);
                let rows = owner.resolve(publication.as_ref(), &unavailable)?;
                AvailableInput::source(
                    Rc::clone(owner),
                    publication.clone(),
                    match source {
                        InputSource::Region(id) => ProducerRoot::Region(id),
                        _ => ProducerRoot::Input(source),
                    },
                    partition.clone(),
                    self.coverage.clone(),
                    rows,
                )
            })
            .clone()
    }
}

impl PublicationScope {
    fn sources(
        &self,
        owner: &Rc<RelationOwner>,
        sources: &[crate::application::file_dag::frontiers::types::FrontierSource],
        rows: Vec<crate::application::file_dag::frontiers::types::FrontierInput>,
    ) -> Result<Vec<Rc<ProjectFrontierPopulation<AvailableInput>>>, OperatingError> {
        rows.into_iter()
            .map(|row| {
                let (input, coverage, payload) = match row {
                    FrontierInput::Decoded { frontier, coverage } => (
                        frontier.input,
                        coverage,
                        ProjectFrontierPayload::Flat(Arc::new(frontier.table)),
                    ),
                    FrontierInput::Compact {
                        input,
                        table,
                        coverage,
                        ..
                    } => (
                        input,
                        coverage,
                        ProjectFrontierPayload::Compact(Arc::new(table)),
                    ),
                };
                let source = sources
                    .iter()
                    .find(|source| source.input == input)
                    .ok_or(OperatingError::NotRead)?
                    .root
                    .clone();
                let populations = &mut *self.populations.borrow_mut();
                populations.source(
                    |found| {
                        if found.source == source
                            && found.coverage == coverage
                            && found.payload == payload
                        {
                            ProjectSourceMatch::Equivalent
                        } else {
                            ProjectSourceMatch::Distinct
                        }
                    },
                    || {
                        Ok(ProjectFrontierPopulation {
                            id: owner.project_identity()?,
                            source: source.clone(),
                            coverage: coverage.clone(),
                            payload: payload.clone(),
                            admitted: std::cell::OnceCell::new(),
                        })
                    },
                )
            })
            .collect()
    }
    fn invocation(
        &self,
        owner: &Rc<RelationOwner>,
        sources: &[Rc<ProjectFrontierPopulation<AvailableInput>>],
        refused: &[crate::domain::crossing::types::RefusedFrontier],
        frontier_sources: &[crate::application::file_dag::frontiers::types::FrontierSource],
        bound: &[Option<crate::application::file_dag::frontiers::types::BoundInput>],
        prepared: &PreparedProgram,
        rule: RuleIndex,
        policy: CompletionPolicy,
    ) -> Result<crate::application::file_dag::derived::owner::PopulationId, OperatingError> {
        let mut bindings = Vec::new();
        for binding in bound.iter().flatten() {
            bindings.push(BoundKey::Input(InputSource::Observation(
                super::source_admission::observation_slot(prepared, rule, binding.input)?,
            )));
            match &binding.members {
                Err(error) => bindings.push(BoundKey::Refused(error.clone())),
                Ok(members) => {
                    for member in members {
                        bindings.push(match member {
                        crate::application::file_dag::frontiers::types::BoundMember::Current(publication) => BoundKey::Current(publication.admitted().population(owner)?, publication.admitted().coverage()?),
                        crate::application::file_dag::frontiers::types::BoundMember::Restored(token) => BoundKey::Restored(token.clone()),
                    });
                    }
                }
            }
        }
        let key = ProjectInvocationKey {
            sources: sources.iter().map(|source| source.id).collect(),
            refused: refused
                .iter()
                .map(|entry| ProjectRefusal {
                    source: frontier_sources
                        .iter()
                        .find(|source| source.input == entry.input)
                        .map(|source| source.root.clone()),
                    input: entry.input,
                    error: entry.error.clone(),
                })
                .collect(),
            policy,
            bound: bindings,
        };
        self.populations
            .borrow_mut()
            .invocation(key, || owner.project_identity())
    }
}

pub(crate) struct ProjectRun<'a> {
    program: &'a Arc<PreparedProgram>,
    services: Arc<ProgramServices>,
    execution: &'a ProgramExecution,
    owner: Rc<RelationOwner>,
    partials: ScopePartials<'a>,
    frontier_producer: RuleIndex,
    frontier_sources: Box<[crate::application::file_dag::frontiers::types::FrontierSource]>,
    ordinary: Vec<Rc<ProjectFrontierPopulation<AvailableInput>>>,
    refused: Vec<crate::domain::crossing::types::RefusedFrontier>,
    bound: Vec<Option<crate::application::file_dag::frontiers::types::BoundInput>>,
    cache: Vec<(
        crate::domain::executable_program::types::InputSource,
        AvailableInput,
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
        let execution = ProgramExecution::open(
            Arc::clone(program.program()),
            Partition::Project(required.clone()),
            Rc::clone(&owner),
        );
        scope
            .execution
            .set(execution)
            .map_err(|_| OperatingError::WrongOperation)?;
    }
    let execution = scope.execution.get().ok_or(OperatingError::NotRun)?;
    let ordinary = scope.sources(&owner, &frontiers.ordinary.sources, frontiers.ordinary.rows)?;
    Ok(ProjectRun {
        program,
        services: Arc::clone(&services),
        execution,
        owner,
        partials,
        frontier_producer: frontiers.ordinary.producer,
        frontier_sources: frontiers.ordinary.sources,
        ordinary,
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
    fn identity(
        &self,
        rule: RuleIndex,
    ) -> Result<crate::application::file_dag::derived::owner::PopulationId, OperatingError> {
        crate::application::file_dag::frontiers::types::validate_consumer_sources(
            self.program,
            self.frontier_producer,
            &self.frontier_sources,
            rule,
        )?;
        self.publication.invocation(
            &self.owner,
            &self.ordinary,
            &self.refused,
            &self.frontier_sources,
            &self.bound,
            self.program,
            rule,
            self.completion(rule)?.for_rule(RuleIndex(0)),
        )
    }
    fn completion(&self, rule: RuleIndex) -> Result<CompletionPolicy, OperatingError> {
        match &self.partials {
            ScopePartials::Project {
                evaluation,
                publication,
            } => Ok(CompletionPolicy::project_inputs(
                rule,
                evaluation,
                publication,
            )),
            ScopePartials::File { .. } => Err(OperatingError::WrongOperation),
        }
    }
    fn roots(
        &self,
        rule: RuleIndex,
        demand: RuleDemand,
    ) -> Result<Vec<crate::domain::executable_program::types::RegionId>, OperatingError> {
        use crate::domain::executable_program::types::InputSource;
        let metadata = self.program.rule_metadata(rule)?;
        let mut roots = Vec::new();
        if demand.emit == EmitDemand::Enabled {
            if let Ok(id) = &metadata.emit_root {
                if self.program.program().regions()[id.0].phase
                    == ryo_values::domain::plans::types::StepScope::CrossFile
                {
                    roots.push(*id);
                }
            }
        }
        if demand.observations == ObservationDemand::Required {
            for root in &metadata.observations {
                if root.phase != ryo_values::domain::plans::types::StepScope::CrossFile {
                    continue;
                }
                match &root.source {
                    Ok(InputSource::Region(id)) => roots.push(*id),
                    Ok(InputSource::Observation(slot)) => {
                        if let Some(Ok(id)) =
                            self.program.program().observation_regions().get(*slot)
                        {
                            roots.push(*id);
                        }
                    }
                    // Direct supplied members have no speculative producer traversal.
                    _ => {}
                }
            }
        }
        Ok(roots)
    }
    pub(crate) fn rule(
        &mut self,
        rule: RuleIndex,
        demand: RuleDemand,
        symbols: &mut ryo_language::domain::symbols::types::Symbols,
    ) -> Result<ProjectRuleOutcome, OperatingError> {
        let roots = self.roots(rule, demand)?;
        let identity = self.identity(rule)?;
        let state = self.execution.project_state(identity);
        self.deliver(&state, rule, demand, &roots, symbols)
    }
    fn deliver(
        &mut self,
        state: &crate::application::file_dag::program::runtime::types::RuntimeState,
        rule: RuleIndex,
        demand: RuleDemand,
        roots: &[crate::domain::executable_program::types::RegionId],
        symbols: &mut ryo_language::domain::symbols::types::Symbols,
    ) -> Result<ProjectRuleOutcome, OperatingError> {
        let completion = self.completion(rule)?;
        let mut context = self.context.borrow(symbols, self.program, rule)?;
        let context = &mut context;
        let metadata = self.program.rule_metadata(rule)?;
        let unavailable = |_relation| Err(OperatingError::NotRead);
        let kernels = super::super::kernel_session::KernelSession::project(
            self.services.kernels(),
            self.program,
            Rc::clone(&self.owner),
            &unavailable,
        );
        let mut absent = HashMap::new();
        let mut sources = SourceInputs {
            execution: self.execution,
            state,
            completion: completion.clone(),
            owner: Rc::clone(&self.owner),
            read: &unavailable,
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
        // One ready cohort contains all actual roots of this compatible population.
        if !roots.is_empty() {
            self.execution.prepare_roots(
                state,
                roots,
                &mut sources,
                &kernels,
                &completion,
                context,
            )?;
        }
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
                    self.execution
                        .invoke(state, *id, &mut sources, &kernels, &completion, context),
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
                let result = root
                    .source
                    .clone()
                    .and_then(|source| {
                        self.execution.admit_root(
                            state,
                            rule,
                            source,
                            &mut sources,
                            &kernels,
                            &completion,
                            context,
                        )
                    })
                    .map(|input| input.with_coverage(completion.coverage(&root.reads)));
                observations.push((
                    ryo_values::domain::observations::types::ObservationIndex(
                        u32::try_from(index).map_err(|_| OperatingError::ShapeMismatch)?,
                    ),
                    match result {
                        Ok(input) => match table_of(&self.owner, &input, &unavailable)
                            .and_then(|table| Ok((table, input.coverage()?)))
                        {
                            Ok((
                                table,
                                crate::domain::completion::types::CoverageState::Complete,
                            )) => ProjectObservation::Complete {
                                table,
                                publication: ObservedPublication::from_admitted(input),
                            },
                            Ok((
                                table,
                                crate::domain::completion::types::CoverageState::Partial { .. },
                            )) => ProjectObservation::Partial {
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
            counts: self.execution.logical_counts(state, rule)?,
        })
    }
}

/// Cohort binding never reads source geometry. Exact immutable source/policy identity
/// selects a shared state; the existing interpreter remains the only evaluator.
#[derive(Clone, Copy)]
pub(crate) struct ProjectRootDeclaration {
    pub(crate) rule: RuleIndex,
    pub(crate) demand: RuleDemand,
}
pub(crate) fn evaluate_roots<E>(
    mut runs: Vec<Result<ProjectRun<'_>, OperatingError>>,
    declarations: &[ProjectRootDeclaration],
    symbols: &mut ryo_language::domain::symbols::types::Symbols,
    mut guard: impl FnMut(RuleIndex) -> Result<(), E>,
) -> crate::application::file_dag::program::protocol::types::ProjectBatch<
    Result<ProjectRuleOutcome, OperatingError>,
    E,
> {
    use crate::domain::executable_program::types::RegionId;
    enum RootSet {
        Pending(Vec<RegionId>),
        Prepared,
    }
    struct Cohort {
        state: Rc<crate::application::file_dag::program::runtime::types::RuntimeState>,
        roots: RootSet,
    }
    let mut cohorts = Vec::new();
    let mut by_identity: HashMap<PopulationId, usize> = HashMap::new();
    let mut memberships = Vec::with_capacity(runs.len());
    for (run, declaration) in runs.iter_mut().zip(declarations) {
        let ProjectRootDeclaration { rule, demand } = *declaration;
        let membership = match run {
            Ok(run) => (|| {
                let identity = run.identity(rule)?;
                let roots = run.roots(rule, demand)?;
                let at = match by_identity.get(&identity) {
                    Some(&at) => at,
                    None => {
                        let at = cohorts.len();
                        cohorts.push(Cohort {
                            state: run.execution.project_state(identity),
                            roots: RootSet::Pending(Vec::new()),
                        });
                        by_identity.insert(identity, at);
                        at
                    }
                };
                if let RootSet::Pending(all) = &mut cohorts[at].roots {
                    all.extend(roots);
                }
                Ok(at)
            })(),
            Err(error) => Err(error.clone()),
        };
        memberships.push(membership);
    }
    let mut completed = Vec::with_capacity(runs.len());
    for ((run, declaration), membership) in runs.into_iter().zip(declarations).zip(memberships) {
        let ProjectRootDeclaration { rule, demand } = *declaration;
        let requires_guard = match &membership {
            Ok(index) => matches!(cohorts[*index].roots, RootSet::Pending(_)),
            Err(_) => true,
        };
        if requires_guard {
            if let Err(error) = guard(rule) {
                return crate::application::file_dag::program::protocol::types::ProjectBatch::Refused { completed, error };
            }
        }
        let result = (|| {
            let mut run = run?;
            let cohort = &mut cohorts[membership?];
            let roots = match std::mem::replace(&mut cohort.roots, RootSet::Prepared) {
                RootSet::Pending(roots) => roots,
                RootSet::Prepared => Vec::new(),
            };
            run.deliver(&cohort.state, rule, demand, &roots, symbols)
        })();
        completed.push(result);
    }
    crate::application::file_dag::program::protocol::types::ProjectBatch::Complete(completed)
}
