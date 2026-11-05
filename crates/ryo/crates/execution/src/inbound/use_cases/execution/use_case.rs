//! Thin public orchestration over the one private execution owner.
use super::{interface::*, types::*};
use crate::application::file_dag::{
    execution::CommonOperatorEngine,
    program::{
        facade,
        facade_runtime::{self, ProgramServices},
    },
    scope::ScopePartials,
};
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
        let inner = facade::prepare(
            compiled,
            &request.rules,
            &request.digests,
            &request.hosted,
            Arc::clone(&self.derived),
            request.observations,
        )?;
        let kernels = Arc::new(CommonOperatorEngine::new(Arc::clone(&self.operators)));
        Ok(PreparedProgram {
            inner,
            kernels,
            derived: Arc::clone(&self.derived),
        })
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
            crate::application::file_dag::frontiers::service::PreparationPolicy::Diagnostic,
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
        let inner = facade_runtime::open_file(
            &self.program,
            input.file,
            ScopePartials::File {
                evaluation: input.evaluation,
                publication: input.publication,
            },
            input.demanded,
            Arc::clone(&self.services),
            input.source,
            symbols,
            input.semantic,
            input.kinds,
        )?;
        Ok(FileSession {
            inner,
            read: input.read,
        })
    }
    fn project_scope(
        &self,
        rule: RuleIndex,
        corpus: &[CorpusInput],
        covered: &[Covered],
        skipped: &[Skipped],
    ) -> Result<ProjectScope, OperatingError> {
        let required = facade_runtime::required_partitions(
            &self.program,
            rule,
            corpus,
            covered,
            skipped,
            &self.services,
        )?;
        let publication = facade_runtime::publication_owner(&self.program, &self.services)?;
        Ok(ProjectScope {
            required,
            publication,
        })
    }
    fn open_project<'a>(
        &'a self,
        input: ProjectInput<'a>,
        symbols: &mut Symbols,
    ) -> Result<ProjectSession<'a>, OperatingError> {
        let frontiers = facade::ProjectFrontiers {
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
        self.inner.rule(rule, demand, self.read, symbols)
    }
    fn capture_frontiers(
        &mut self,
        rule: RuleIndex,
        first: Bucket,
        symbols: &mut Symbols,
        capture: CaptureContext<'_>,
    ) -> Result<Vec<FrontierCapture>, OperatingError> {
        self.inner
            .capture_frontiers(rule, first, self.read, symbols, capture)
    }
    fn commit_frontier(
        &mut self,
        rule: RuleIndex,
        input: ryo_values::domain::plan_parts::types::InputNumber,
    ) -> Result<(), OperatingError> {
        self.inner.commit_frontier(rule, input)
    }
    fn skip_rule(
        &mut self,
        rule: RuleIndex,
        reason: ApplicabilityReason,
    ) -> Result<(), OperatingError> {
        self.inner.skip_rule(rule, reason)
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

impl ProjectStage for ProjectScope {
    fn finish(self) -> Result<(), OperatingError> {
        self.publication.finish()
    }
}
