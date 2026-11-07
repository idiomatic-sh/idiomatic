//! The engine's compilation and execution as the use cases call them: the kernel's implementations, held here where an application
//! service may name them, so a use case builds none.

use std::sync::Arc;

use ryo_execution::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use ryo_execution::application::file_dag::program::request_execution::types::RequestExecution;
use ryo_execution::application::rule_compilation::errors::RuleCompilationError;
use ryo_execution::application::rule_compilation::service::{
    CompiledPlans, CompiledQueries, ExecutionCompilation, QueryCompilation,
};
use ryo_execution::application::rule_compilation::types::PreparedRuleIdentities;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution::inbound::use_cases::prepare::types::PrepareRequest;
use ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use ryo_execution::outbound::derived::interface::DerivedPort;
use ryo_execution::outbound::digest::interface::DigestPort;
use ryo_execution::outbound::operators::interface::OperatorsPort;
use ryo_execution::outbound::query_language::interface::QueryLanguagePort;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;

/// The engine's rule compilation, with the plans it keeps.
pub struct Compilation(QueryCompilation);

impl std::fmt::Debug for Compilation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl Compilation {
    /// Compilation over the query language the edge injects.
    #[must_use]
    pub fn new(compiler: Arc<dyn QueryLanguagePort>) -> Self {
        Self(QueryCompilation::new(compiler))
    }
}

impl ExecutionCompilation for Compilation {
    fn compiling_into(self, cache: CompiledPlans) -> Self {
        Self(self.0.compiling_into(cache))
    }

    fn compile(
        &self,
        rules: &[ryo_values::domain::rule_requests::types::RuleSpec],
        hosted: &[ryo_values::domain::hosted::types::HostRelation],
        digest: &dyn DigestPort,
    ) -> Result<CompiledQueries, RuleCompilationError> {
        self.0.compile(rules, hosted, digest)
    }

    fn identify(
        &self,
        queries: &CompiledQueries,
        digest: &dyn DigestPort,
        rules: Vec<ryo_values::domain::stores::types::StoredRule>,
    ) -> Result<PreparedRuleIdentities, RuleCompilationError> {
        self.0.identify(queries, digest, rules)
    }
}

/// The engine's preparation of a request and the beginning of its execution.
pub struct Execution(PrepareUseCaseImpl);

impl std::fmt::Debug for Execution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl Execution {
    /// Execution over the operators and the derived views the edge injects.
    #[must_use]
    pub fn new(operators: Arc<dyn OperatorsPort>, derived: Arc<dyn DerivedPort>) -> Self {
        Self(PrepareUseCaseImpl::new(operators, derived))
    }

    /// Begins the execution of a prepared program over the corpus it runs on.
    #[must_use]
    pub fn begin(
        &self,
        program: &PreparedExecutionProgram,
        corpus: &[ryo_values::domain::files::types::CorpusInput],
    ) -> RequestExecution {
        ExecutionUseCaseImpl::new().execution(program, corpus)
    }
}

impl PrepareUseCase for Execution {
    fn prepare(&self, request: PrepareRequest) -> Result<PreparedExecutionProgram, OperatingError> {
        self.0.prepare(request)
    }
}
