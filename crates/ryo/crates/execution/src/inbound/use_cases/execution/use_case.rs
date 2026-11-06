//! Thin public orchestration over the one private execution owner.
use super::{interface::*, types::*};
use crate::application::file_dag::execution::CommonOperatorEngine;
use crate::application::file_dag::program::facade_runtime;
use crate::application::file_dag::program::facade_runtime::ProgramServices;
use crate::application::file_dag::scope::ScopePartials;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::{
    files::types::CorpusInput, runs::types::Skipped, stored_facts::types::Bucket,
};
use std::sync::Arc;
pub struct ExecutionUseCaseImpl {
    operators: Arc<dyn OperatorsPort>,
    derived: Arc<dyn DerivedPort>,
}
impl ExecutionUseCaseImpl {
    pub fn new(operators: Arc<dyn OperatorsPort>, derived: Arc<dyn DerivedPort>) -> Self {
        Self { operators, derived }
    }
}
impl ExecutionUseCase for ExecutionUseCaseImpl {
    fn prepare(&self, request: PreparedRequest) -> Result<PreparedProgram, OperatingError> {
        let compiled = crate::domain::prepared_request::service::assemble(
            request.plans,
            request.bindings,
            &request.rules,
        )?;
        self.finish_preparation(
            compiled,
            request.rules,
            request.digests,
            request.hosted,
            request.observations,
        )
    }
    fn prepare_compiled(
        &self,
        queries: CompiledQueries,
        rules: Vec<ryo_values::domain::rule_requests::types::RuleSpec>,
        digests: Vec<ryo_values::domain::stored_facts::types::Digest>,
        hosted: Vec<ryo_values::domain::hosted::types::HostRelation>,
        observations: ObservationIdentity,
    ) -> Result<PreparedProgram, OperatingError> {
        self.finish_preparation(queries.inner, rules, digests, hosted, observations)
    }
    fn begin(&self, program: &PreparedProgram, corpus: &[CorpusInput]) -> RequestExecution {
        RequestExecution {
            program: Arc::clone(&program.inner),
            services: Arc::new(ProgramServices::new(
                Arc::clone(&program.kernels),
                Arc::clone(&program.derived),
                corpus,
            )),
        }
    }
}
impl ExecutionRequest for RequestExecution {
    fn observations<'a>(
        &'a self,
        arenas: &'a mut Vec<ryo_values::domain::stored_facts::types::ArenaBytes>,
    ) -> Result<ObservationStage<'a>, OperatingError> {
        let metadata = self
            .program
            .observation_metadata()
            .ok_or(OperatingError::NotRun)?;
        let phase = self.project_phase()?;
        let publications =
            crate::application::file_dag::observations::cross::CrossPublications::new(
                metadata,
                &phase.publication,
                arenas,
            );
        Ok(ObservationStage {
            phase,
            publications,
        })
    }
    fn prepare_frontiers(
        &self,
        input: FrontierRequest<'_>,
        symbols: &mut Symbols,
    ) -> Result<PreparedFrontiers, PreparationError> {
        crate::application::file_dag::frontiers::service::prepare(
            &self.program,
            input.rule,
            input.first,
            input.facts,
            input.project,
            input.project_symbols,
            symbols,
            &input.scope.required,
            match input.demand { FrontierDemand::Diagnostic => crate::application::file_dag::frontiers::service::PreparationPolicy::Diagnostic, FrontierDemand::Observations { ready } => crate::application::file_dag::frontiers::service::PreparationPolicy::Observations { ready } },
        )
    }
    fn finish(self) {
        drop(self);
    }

    fn open_file<'a, 'facts: 'a>(
        &'a self,
        input: FileInput<'a, 'facts>,
        symbols: &mut Symbols,
    ) -> Result<FileSession<'a, 'facts>, OperatingError> {
        let mut vocabulary = input.kinds;
        let vocabulary = crate::domain::kind_sets::service::prepare_vocabulary(
            &mut vocabulary,
            input.elements,
            symbols,
            ryo_values::domain::files::types::SourceView(input.source),
        );
        let inner = facade_runtime::open_file(
            &self.program,
            input.file,
            ScopePartials::File {
                evaluation: input.evaluation,
                publication: input.publication,
            },
            input.intents,
            Arc::clone(&self.services),
            input.source,
            matches!(input.language, SourceLanguage::Recognized)
                && input.resident_stability == facts::ResidentStability::Stable,
            symbols,
            input.semantic,
            vocabulary,
        )?;
        Ok(FileSession {
            inner,
            program: Arc::clone(&self.program),
            read: input.read,
            elements: input.elements,
            file: input.file,
        })
    }
    fn project_phase(&self) -> Result<ProjectPhase<'_>, OperatingError> {
        Ok(ProjectPhase {
            request: self,
            publication: std::rc::Rc::new(facade_runtime::publication_owner(
                &self.program,
                &self.services,
            )?),
        })
    }
    fn open_project<'a>(
        &'a self,
        input: ProjectInput<'a>,
        symbols: &mut Symbols,
    ) -> Result<ProjectSession<'a>, OperatingError> {
        let frontiers = crate::application::file_dag::program::facade_runtime::ProjectFrontiers {
            publication: &input.scope.publication,
            ordinary: input.ordinary,
            bound_inputs: input.bound,
        };
        let inner = facade_runtime::open_project(
            &self.program,
            input.scope.required.clone(),
            ScopePartials::Project {
                evaluation: input.evaluation,
                publication: input.publication,
            },
            frontiers,
            symbols,
            input.arenas,
            Arc::clone(&self.services),
        )?;
        Ok(ProjectSession { inner })
    }
}
impl FileExecution for FileSession<'_, '_> {
    fn readiness_counts(&self) -> ReadinessCounts {
        self.inner.readiness_counts()
    }

    fn walk_counts(&self) -> WalkCounts {
        self.inner.walk_counts()
    }

    fn emit_rows(
        &mut self,
        rule: RuleIndex,
        symbols: &mut Symbols,
        each: &mut dyn FnMut(
            &[ryo_relations::domain::relations::types::Value],
        ) -> Result<(), OperatingError>,
    ) -> Result<RowEmission, OperatingError> {
        let read = |relation| source_rows(self.read, self.elements, self.file, relation);
        self.inner.emit_rows(rule, &read, symbols, each)
    }
    fn finish(self) -> Result<FileFinished, OperatingError> {
        Ok(FileFinished {
            readiness: self.inner.finish()?,
        })
    }

    fn rule(
        &mut self,
        rule: RuleIndex,
        demand: RuleDemand,
        symbols: &mut Symbols,
    ) -> Result<RuleOutcome, OperatingError> {
        let read = |relation| source_rows(self.read, self.elements, self.file, relation);
        self.inner.rule(rule, demand, &read, symbols)
    }
    fn capture_frontiers(
        &mut self,
        rule: RuleIndex,
        first: Bucket,
        symbols: &mut Symbols,
        capture: CaptureContext<'_>,
    ) -> Result<Vec<FrontierCapture>, OperatingError> {
        let read = |relation| source_rows(self.read, self.elements, self.file, relation);
        let bound = crate::application::file_dag::program::protocol::types::CaptureContext {
            file: self.file,
            rule_digest: self.program.rule_digest(rule)?,
            worker: capture.worker,
            retained: crate::domain::finish::types::Retained {
                arena: crate::domain::finish::types::Arena(capture.arena),
                places: capture.places,
            },
            words: capture.words,
        };
        self.inner
            .capture_frontiers(rule, first, &read, symbols, bound)
    }
    fn commit_frontier(
        &mut self,
        rule: RuleIndex,
        input: ryo_values::domain::plan_parts::types::InputNumber,
    ) -> Result<(), OperatingError> {
        self.inner.commit_frontier(rule, input)
    }
    fn counts(&self, rule: RuleIndex) -> Result<LogicalCounts, OperatingError> {
        self.inner.counts(rule)
    }
}
impl ProjectExecution for ProjectSession<'_> {
    fn finish(self) {
        drop(self);
    }

    fn rule(
        &mut self,
        rule: RuleIndex,
        demand: RuleDemand,
        symbols: &mut Symbols,
    ) -> Result<ProjectRuleOutcome, OperatingError> {
        self.inner.rule(rule, demand, symbols)
    }
}

impl ProjectRootExecution for ProjectPhase<'_> {
    fn evaluate<E>(
        &self,
        tasks: Vec<ProjectTask<'_>>,
        symbols: &mut Symbols,
        guard: impl FnMut(RuleIndex) -> Result<(), E>,
    ) -> ProjectBatch<Result<ProjectRuleOutcome, OperatingError>, E> {
        let mut roots = Vec::with_capacity(tasks.len());
        for task in tasks {
            let run = if std::rc::Rc::ptr_eq(&self.publication, &task.input.scope.publication) {
                self.request
                    .open_project(task.input, symbols)
                    .map(|session| session.inner)
            } else {
                Err(OperatingError::NotRead)
            };
            roots.push(facade_runtime::ProjectRoot {
                rule: task.rule,
                demand: task.demand,
                run,
            });
        }
        facade_runtime::evaluate_roots(roots, symbols, guard)
    }
    fn readiness_counts(&self) -> ReadinessCounts {
        self.publication.readiness_counts()
    }
}
impl ProjectStage for ProjectPhase<'_> {
    fn state_counts(&self) -> InvocationStateCounts {
        self.publication.state_counts()
    }
    fn project_scope(
        &self,
        rule: RuleIndex,
        corpus: &[CorpusInput],
        covered: &[Covered],
        skipped: &[Skipped],
    ) -> Result<ProjectScope, OperatingError> {
        let required = facade_runtime::required_partitions(
            &self.request.program,
            rule,
            corpus,
            covered,
            skipped,
            &self.request.services,
        )?;
        Ok(ProjectScope {
            required,
            publication: std::rc::Rc::clone(&self.publication),
        })
    }
    fn finish(self) -> Result<(), OperatingError> {
        match std::rc::Rc::try_unwrap(self.publication) {
            Ok(publication) => publication.finish(),
            Err(shared) => {
                drop(shared);
                Err(OperatingError::NotRead)
            }
        }
    }
}

impl ProjectRootExecution for ObservationStage<'_> {
    fn evaluate<E>(
        &self,
        tasks: Vec<ProjectTask<'_>>,
        symbols: &mut Symbols,
        guard: impl FnMut(RuleIndex) -> Result<(), E>,
    ) -> ProjectBatch<Result<ProjectRuleOutcome, OperatingError>, E> {
        self.phase.evaluate(tasks, symbols, guard)
    }
    fn readiness_counts(&self) -> ReadinessCounts {
        self.phase.readiness_counts()
    }
}

fn source_rows<'a, 'facts: 'a>(
    read: &dyn Fn(
        ryo_relations::domain::relations::types::Relation,
    ) -> Result<SourceRows<'facts>, OperatingError>,
    elements: &'a ryo_language::domain::elements::types::Elements,
    file: ryo_relations::domain::relations::types::FileId,
    relation: ryo_relations::domain::relations::types::Relation,
) -> Result<crate::domain::table_model::types::Rows<'a>, OperatingError> {
    use crate::domain::{
        table_cells::{service::native_form, types::TreeRows},
        table_model::types::Rows,
    };
    if let Some(form) = native_form(relation) {
        return Ok(Rows::Tree(TreeRows {
            form,
            elements,
            file,
        }));
    }
    read(relation).map(|rows| match rows {
        SourceRows::Stored(columns) => Rows::Stored(columns),
        SourceRows::Computed(table) => Rows::Computed(table),
    })
}

impl PreparedExecution for PreparedProgram {
    fn observation_batches(&self) -> Option<&[Vec<RuleIndex>]> {
        self.inner.observation_batches()
    }
    fn rule_count(&self) -> usize {
        self.inner.rule_count()
    }
    fn rule_order(&self) -> impl Iterator<Item = RuleIndex> + '_ {
        self.inner.rule_order()
    }
    fn demanded_rules(&self, governs: impl Fn(usize) -> bool) -> Vec<bool> {
        self.inner.demanded_rules(governs)
    }
    fn emit(
        &self,
        rule: RuleIndex,
    ) -> Result<&ryo_values::domain::plans::types::Emit, OperatingError> {
        self.inner.emit(rule)
    }
    fn rule_digest(
        &self,
        rule: RuleIndex,
    ) -> Result<ryo_values::domain::stored_facts::types::Digest, OperatingError> {
        self.inner.rule_digest(rule)
    }
    fn frontiers(
        &self,
        rule: RuleIndex,
    ) -> Result<&[ryo_values::domain::plan_parts::types::Input], OperatingError> {
        self.inner.frontiers(rule)
    }
    fn captures_observation(&self, rule: RuleIndex, input: usize) -> Result<bool, OperatingError> {
        self.inner.captures_observation(rule, input)
    }
    fn semantic_needed(&self) -> bool {
        self.inner.semantic_needed()
    }
    fn observation_layout(
        &self,
    ) -> Option<(
        ryo_values::domain::observations::types::ObservationReadinessLayout,
        &[ObservedFrontierRoot],
    )> {
        self.inner.observation_layout()
    }
    fn observation_reads(&self, rule: RuleIndex) -> Result<Vec<RelationRead>, OperatingError> {
        self.inner.observation_reads(rule)
    }
    fn has_observation_contract(&self, rule: RuleIndex) -> Result<bool, OperatingError> {
        self.inner.has_observation_contract(rule)
    }
    fn frontier_dependencies(
        &self,
        rule: RuleIndex,
    ) -> Result<&[Vec<ryo_values::domain::observations::types::ObservationBindingId>], OperatingError>
    {
        self.inner.frontier_dependencies(rule)
    }
}

/// Query compilation is a preparation use case, not a second execution pipeline.
pub struct CompilationUseCaseImpl {
    inner: crate::application::rule_compilation::service::RuleCompilation,
}
impl CompilationUseCaseImpl {
    pub fn new(compiler: Arc<dyn query_language::QueryLanguagePort>) -> Self {
        Self {
            inner: crate::application::rule_compilation::service::RuleCompilation::new(compiler),
        }
    }
}
impl ExecutionCompilation for CompilationUseCaseImpl {
    fn compiling_into(self, cache: CompiledPlans) -> Self {
        Self {
            inner: self.inner.compiling_into(cache),
        }
    }
    fn compile(
        &self,
        rules: &[ryo_values::domain::rule_requests::types::RuleSpec],
        hosted: &[ryo_values::domain::hosted::types::HostRelation],
    ) -> Result<CompiledQueries, RuleCompilationError> {
        let inner = self.inner.prepare(rules, hosted)?;
        crate::application::rule_compilation::service::observed(
            rules,
            &inner.plans,
            &inner.bindings,
            hosted,
        );
        Ok(CompiledQueries { inner })
    }
    fn identify(
        &self,
        queries: &CompiledQueries,
        digest: &dyn digest::DigestPort,
        rules: Vec<ryo_values::domain::stores::types::StoredRule>,
    ) -> Result<PreparedRuleIdentities, RuleCompilationError> {
        let identities = crate::application::rule_compilation::identity::with_producers(
            digest,
            rules,
            &queries.inner.plans,
            &queries.inner.schedule,
            &queries.inner.observations,
        )?;
        Ok(PreparedRuleIdentities {
            rules: identities.rules,
            observations: identities.observations,
        })
    }
}
impl PreparedQueries for CompiledQueries {
    fn plans(&self) -> &[ryo_values::domain::plans::types::Plan] {
        &self.inner.plans
    }
    fn bindings(&self) -> &[Vec<ryo_values::domain::query_bindings::types::ParameterBinding>] {
        &self.inner.bindings
    }
}

impl ObservationExecution for ObservationStage<'_> {
    fn reserve(&mut self, rule: RuleIndex) -> Result<PublicationSlot, OperatingError> {
        self.publications.reserve(rule)
    }
    fn bindings(&self, rule: RuleIndex) -> Result<Vec<BoundInput>, OperatingError> {
        self.publications.bindings(rule)
    }
    fn binding_readiness(
        &self,
        rule: RuleIndex,
    ) -> Result<
        std::collections::BTreeMap<
            ryo_values::domain::observations::types::ObservationBindingId,
            bool,
        >,
        OperatingError,
    > {
        self.publications.binding_readiness(rule)
    }
    fn arenas(&self) -> &[ryo_values::domain::stored_facts::types::ArenaBytes] {
        self.publications.arenas()
    }
    fn project_scope(
        &self,
        rule: RuleIndex,
        corpus: &[CorpusInput],
        covered: &[Covered],
        skipped: &[Skipped],
    ) -> Result<ProjectScope, OperatingError> {
        self.phase.project_scope(rule, corpus, covered, skipped)
    }
    fn finish(&mut self, rule: RuleIndex) -> Result<(), OperatingError> {
        self.publications.finish(rule)
    }
    fn finish_stage(self) -> Result<(), OperatingError> {
        drop(self.publications);
        self.phase.finish()
    }
}

impl ObservationPublication for ObservationStage<'_> {
    fn install(
        &mut self,
        rule: RuleIndex,
        slot: PublicationSlot,
        transfer: CrossPublicationTransfer,
        publications: Vec<(
            ryo_values::domain::observations::types::ObservationIndex,
            ObservedPublication,
        )>,
    ) -> Result<(), OperatingError> {
        self.publications
            .install(rule, slot, transfer, publications)
    }
    fn restore_frontiers(
        &mut self,
        facts: &dyn facts::FactsFinish,
        corpus: &[CorpusInput],
        covered: &[Covered],
        skipped: &[Skipped],
        frames: &[FileObservationReadiness],
        first_bucket: &[Bucket],
        selected: &[bool],
    ) -> Result<(), OperatingError> {
        crate::application::file_dag::observations::cross::restore_frontiers(
            &mut self.publications,
            &self.phase.request.program,
            &self.phase.request.services,
            &self.phase.publication,
            facts,
            corpus,
            covered,
            skipped,
            frames,
            first_bucket,
            selected,
        )
    }
}

impl ExecutionUseCaseImpl {
    fn finish_preparation(
        &self,
        compiled: crate::domain::prepared_request::types::Compiled,
        rules: Vec<ryo_values::domain::rule_requests::types::RuleSpec>,
        digests: Vec<ryo_values::domain::stored_facts::types::Digest>,
        hosted: Vec<ryo_values::domain::hosted::types::HostRelation>,
        identity: ObservationIdentity,
    ) -> Result<PreparedProgram, OperatingError> {
        let inner = crate::application::file_dag::program::preparation::service::prepare(
            compiled,
            &rules,
            &digests,
            &hosted,
            Arc::clone(&self.derived),
            identity,
        )?;
        Ok(PreparedProgram {
            inner,
            kernels: Arc::new(CommonOperatorEngine::new(Arc::clone(&self.operators))),
            derived: Arc::clone(&self.derived),
        })
    }
}

impl PreparedReporting for PreparedProgram {
    fn metadata(&self) -> &[ryo_values::domain::run_metadata::types::RuleMetadata] {
        self.inner.metadata()
    }
    fn replay_closure(
        &self,
        again: ryo_values::domain::stores::types::Reuse,
        first: &[Bucket],
    ) -> Result<(ryo_values::domain::stores::types::Reuse, Vec<Bucket>), OperatingError> {
        self.inner.replay_closure(again, first)
    }
}
