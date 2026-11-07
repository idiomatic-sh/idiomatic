//! The engine's compilation and execution as the use cases call them: the kernel's implementations, held here where an application
//! service may name them, so a use case builds none.

use std::sync::Arc;

use ryo_execution::inbound::use_cases::execution::interface::{ExecutionCompilation, ExecutionUseCase};
use ryo_execution::inbound::use_cases::execution::types::{
    CompiledPlans, CompiledQueries, DerivedPort, ObservationIdentity, OperatingError,
    OperatorsPort, PreparedExecutionProgram, PreparedRuleIdentities, RequestExecution,
    RuleCompilationError,
};
use ryo_execution::inbound::use_cases::execution::types::digest::DigestPort;
use ryo_execution::inbound::use_cases::execution::types::query_language::QueryLanguagePort;
use ryo_execution::inbound::use_cases::execution::use_case::{
    CompilationUseCaseImpl, ExecutionUseCaseImpl,
};

/// The engine's rule compilation, with the plans it keeps.
pub struct Compilation(CompilationUseCaseImpl);

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
        Self(CompilationUseCaseImpl::new(compiler))
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
    ) -> Result<CompiledQueries, RuleCompilationError> {
        self.0.compile(rules, hosted)
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

/// The engine's preparation and execution of a request.
pub struct Execution(ExecutionUseCaseImpl);

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
        Self(ExecutionUseCaseImpl::new(operators, derived))
    }

    /// The same, wiring only the strategies `routes` names, for a differential run.
    #[cfg(feature = "fixtures")]
    #[must_use]
    pub const fn routing(
        self,
        routes: ryo_execution::inbound::use_cases::execution::types::RoutePolicy,
    ) -> Self {
        Self(self.0.routing(routes))
    }
}

impl ExecutionUseCase for Execution {
    fn prepare(
        &self,
        request: ryo_execution::inbound::use_cases::execution::types::PreparedRequest,
    ) -> Result<PreparedExecutionProgram, OperatingError> {
        self.0.prepare(request)
    }

    fn prepare_compiled(
        &self,
        queries: CompiledQueries,
        rules: Vec<ryo_values::domain::rule_requests::types::RuleSpec>,
        digests: Vec<ryo_values::domain::stored_facts::types::Digest>,
        hosted: Vec<ryo_values::domain::hosted::types::HostRelation>,
        observations: ObservationIdentity,
    ) -> Result<PreparedExecutionProgram, OperatingError> {
        self.0
            .prepare_compiled(queries, rules, digests, hosted, observations)
    }

    fn begin(
        &self,
        program: &PreparedExecutionProgram,
        corpus: &[ryo_values::domain::files::types::CorpusInput],
    ) -> RequestExecution {
        self.0.begin(program, corpus)
    }
}
