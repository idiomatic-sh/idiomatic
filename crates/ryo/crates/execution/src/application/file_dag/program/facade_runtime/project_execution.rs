//! Project execution over deferred frontier sources and one publication scope.
use super::super::project_population::types::ProjectFrontierPopulation;
use crate::application::file_dag::derived::population::types::PopulationId;
use crate::application::file_dag::program::admission::AvailableInput;
use crate::application::file_dag::program::admission::Partition;
use crate::application::file_dag::program::admission::RequiredPartitions;
use crate::application::file_dag::program::facade_runtime::types::ProgramServices;
use crate::application::file_dag::program::facade_runtime::types::ProjectFrontiers;
use crate::application::file_dag::program::facade_runtime::types::PublicationScope;
use crate::application::file_dag::program::native_walk::Deliveries;
use crate::application::file_dag::program::preparation::types::PreparedProgram;
use crate::application::file_dag::program::protocol::types::EmitDemand;
use crate::application::file_dag::program::protocol::types::ObservationDemand;
use crate::application::file_dag::program::protocol::types::ProjectRuleOutcome;
use crate::application::file_dag::program::protocol::types::RuleDemand;
use crate::application::file_dag::program::runtime::service::ProgramExecution;
use crate::application::file_dag::relations::RelationOwner;
use crate::application::file_dag::scope::ScopePartials;
use crate::domain::completion::types::completion_policy::value::CompletionPolicy;
use crate::domain::coverage::types::RuleIndex;
use crate::domain::executable_program::types::InputSource;
use crate::domain::operating::errors::OperatingError;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
mod deliver;
mod sources;

pub(crate) struct ProjectRun<'a> {
    program: &'a Arc<PreparedProgram>,
    services: Arc<ProgramServices>,
    execution: &'a ProgramExecution,
    owner: Rc<RelationOwner>,
    partials: ScopePartials<'a>,
    frontier_producer: RuleIndex,
    frontier_sources: Arc<[crate::application::file_dag::frontiers::types::FrontierSource]>,
    ordinary: Vec<Rc<ProjectFrontierPopulation<AvailableInput>>>,
    refused: Vec<crate::domain::crossing::types::RefusedFrontier>,
    bound: Vec<Option<crate::application::file_dag::frontiers::types::BoundInput>>,
    cache: Vec<crate::application::file_dag::program::facade_runtime::types::CachedInput>,
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
    // The first demand of the frontiers: what preparation left packed is decoded here, before
    // the spellings are checked, so a decode refusal is recorded exactly where it used to be.
    crate::application::file_dag::frontiers::service::admit_all(&mut frontiers.ordinary, symbols);
    crate::domain::spelling::service::check_name_spellings(
        frontiers.ordinary.rows.iter().flat_map(|input| -> Box<dyn Iterator<Item = ryo_relations::domain::relations::types::Name> + '_> {
            match input {
                crate::application::file_dag::frontiers::types::FrontierInput::Decoded { frontier, .. } => Box::new(frontier.table.cells.iter().filter_map(|value| match value { ryo_relations::domain::relations::types::Value::Name(name) => Some(*name), _ => None })),
                crate::application::file_dag::frontiers::types::FrontierInput::Compact { spelling, .. } => Box::new(spelling.iter().copied()),
                crate::application::file_dag::frontiers::types::FrontierInput::Packed(_) => Box::new(std::iter::empty()),
            }
        }), symbols, arenas,
    )?;
    let scope = frontiers.publication;
    scope.validate_request(&services, &required)?;
    let transport = scope.claim_symbols(symbols)?;
    let owner = Rc::clone(scope.owner());
    if let Some(existing) = scope.execution.get() {
        existing.validate_project(program.program(), &required)?;
    } else {
        let execution = ProgramExecution::open(
            Arc::clone(program.program()),
            Partition::Project(required.identity()),
            Rc::clone(&owner),
        );
        scope
            .execution
            .set(execution)
            .map_err(|_| OperatingError::WrongOperation)?;
    }
    let execution = scope.execution.get().ok_or(OperatingError::NotRun)?;
    let ordinary = scope.sources(
        &owner,
        &frontiers.ordinary.sources,
        &frontiers.ordinary.rows,
    )?;
    Ok(ProjectRun {
        program,
        services: Arc::clone(&services),
        execution,
        owner,
        partials,
        frontier_producer: frontiers.ordinary.producer,
        frontier_sources: frontiers.ordinary.sources,
        ordinary,
        refused: frontiers.ordinary.refused.to_vec(),
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
    ) -> Result<
        crate::application::file_dag::derived::population::types::PopulationId,
        OperatingError,
    > {
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
    /// The existing runtime observer receives this run's actual bound identity.
    #[cfg(test)]
    pub(super) fn observe_producer_releases(
        &self,
        rule: RuleIndex,
        producer: crate::domain::executable_program::types::RegionId,
        consumers: &[crate::domain::executable_program::types::RegionId],
    ) -> Result<(), OperatingError> {
        let state = self.execution.project_state(self.identity(rule)?);
        self.execution
            .observe_producer_releases(&state, producer, consumers)
    }
    #[cfg(test)]
    pub(super) fn take_producer_releases(
        &self,
        rule: RuleIndex,
    ) -> Result<Vec<super::super::runtime::types::ProducerLeaseEvent>, OperatingError> {
        let state = self.execution.project_state(self.identity(rule)?);
        Ok(self.execution.take_producer_releases(&state))
    }
    /// The rule is a declared root of its cohort: it owes the project walk one delivery, and
    /// only a declared rule's project-stage classes are admitted to the walk.
    fn declare(&self, rule: RuleIndex) {
        self.publication.walk.owe(self.program, rule, Deliveries(1));
    }
    /// The rule's delivery ended, whatever its outcome: its classes are no longer needed.
    fn delivered(&self, rule: RuleIndex) {
        self.publication.walk.delivered(self.program, rule);
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
                    == ryo_values::domain::plan_parts::types::StepScope::CrossFile
                {
                    roots.push(*id);
                }
            }
        }
        if demand.observations == ObservationDemand::Required {
            for root in &metadata.observations {
                if root.phase != ryo_values::domain::plan_parts::types::StepScope::CrossFile {
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
    #[cfg(test)]
    pub(crate) fn rule(
        &mut self,
        rule: RuleIndex,
        demand: RuleDemand,
        symbols: &mut ryo_language::domain::symbols::types::Symbols,
    ) -> Result<ProjectRuleOutcome, OperatingError> {
        let roots = self.roots(rule, demand)?;
        let identity = self.identity(rule)?;
        let state = self.execution.project_state(identity);
        if !roots.is_empty() {
            self.declare(rule);
        }
        let outcome = self.deliver(&state, rule, demand, &roots, symbols);
        self.delivered(rule);
        outcome
    }
}

/// Cohort binding never reads source geometry. Exact immutable source/policy identity
/// selects a shared state; the existing interpreter remains the only evaluator.
/// A declared root travels with its own run, so a submission cannot omit or add a
/// run without also omitting or adding its declaration.
pub(crate) struct ProjectRoot<'a> {
    pub(crate) rule: RuleIndex,
    pub(crate) demand: RuleDemand,
    pub(crate) run: Result<ProjectRun<'a>, OperatingError>,
}
pub(crate) fn evaluate_roots<E>(
    mut roots: Vec<ProjectRoot<'_>>,
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
        state: Rc<crate::application::file_dag::program::runtime::service::RuntimeState>,
        roots: RootSet,
    }
    // The walk's batches end with this evaluation on every exit: completion, error or a
    // refusing guard that leaves later roots undelivered.
    struct Retire<'scope>(Vec<&'scope PublicationScope>);
    impl Drop for Retire<'_> {
        fn drop(&mut self) {
            for scope in &self.0 {
                scope.walk.retire_all();
            }
        }
    }
    let _retire = Retire(
        roots
            .iter()
            .filter_map(|root| root.run.as_ref().ok().map(|run| run.publication))
            .collect(),
    );
    let mut cohorts = Vec::new();
    let mut by_identity: HashMap<PopulationId, usize> = HashMap::new();
    let mut memberships = Vec::with_capacity(roots.len());
    for ProjectRoot { rule, demand, run } in roots.iter_mut() {
        let (rule, demand) = (*rule, *demand);
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
                if !roots.is_empty() {
                    run.declare(rule);
                }
                if let RootSet::Pending(all) = &mut cohorts[at].roots {
                    all.extend(roots);
                }
                Ok(at)
            })(),
            Err(error) => Err(error.clone()),
        };
        memberships.push(membership);
    }
    let mut completed = Vec::with_capacity(roots.len());
    for (ProjectRoot { rule, demand, run }, membership) in roots.into_iter().zip(memberships) {
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
            let outcome = run.deliver(&cohort.state, rule, demand, &roots, symbols);
            run.delivered(rule);
            outcome
        })();
        completed.push(result);
    }
    crate::application::file_dag::program::protocol::types::ProjectBatch::Complete(completed)
}
