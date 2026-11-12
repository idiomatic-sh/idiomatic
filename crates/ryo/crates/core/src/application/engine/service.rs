//! The engine's compilation and execution as the use cases call them: the kernel's implementations, held here where an application
//! service may name them, so a use case builds none.

use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase as _;
use std::sync::Arc;

use ryo_execution::inbound::use_cases::compile_rules::interface::{
    CompileRulesUseCase, CompiledPlans, CompiledRules, RuleCompilationError,
};
use ryo_execution::inbound::use_cases::compile_rules::types::CompileRequest;
use ryo_execution::inbound::use_cases::compile_rules::use_case::CompileRulesUseCaseImpl;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::PreparedExecutionProgram;
use ryo_execution::inbound::use_cases::execution::interface::RequestExecution;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution::inbound::use_cases::prepare::types::{DerivedViews, PrepareRequest};
use ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use ryo_execution::outbound::query_language::interface::QueryLanguagePort;
use ryo_execution_runtime::outbound::digest::interface::DigestPort;
use ryo_execution_runtime::outbound::operators::interface::OperatorsPort;

/// The engine's rule compilation, with the plans it keeps.
pub(crate) struct Compilation(CompileRulesUseCaseImpl);

impl std::fmt::Debug for Compilation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl Compilation {
    /// Compilation over the query language and the digest the edge injects.
    #[must_use]
    pub(crate) fn new(compiler: Arc<dyn QueryLanguagePort>, digest: Arc<dyn DigestPort>) -> Self {
        Self(CompileRulesUseCaseImpl::new(compiler, digest))
    }

    /// The same, keeping the plans it compiles in `plans`, which other use cases share.
    #[must_use]
    pub(crate) fn compiling_into(self, plans: CompiledPlans) -> Self {
        Self(self.0.compiling_into(plans))
    }
}

impl CompileRulesUseCase for Compilation {
    fn compile_rules(
        &self,
        request: CompileRequest<'_>,
    ) -> Result<CompiledRules, RuleCompilationError> {
        self.0.compile_rules(request)
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
    /// Execution over the operators, the derived views and the digest the edge injects.
    #[must_use]
    pub fn new(
        operators: Arc<dyn OperatorsPort>,
        derived: DerivedViews,
        digest: Arc<dyn DigestPort>,
    ) -> Self {
        Self(PrepareUseCaseImpl::new(operators, derived, digest))
    }

    /// Begins the execution of a prepared program over the corpus it runs on, telling `observer` of the engine's work.
    #[must_use]
    pub fn begin(
        &self,
        program: &PreparedExecutionProgram,
        corpus: &[ryo_values::domain::files::types::CorpusInput],
        observer: Option<
            Arc<dyn ryo_execution_runtime::outbound::observer::interface::ObserverPort>,
        >,
    ) -> RequestExecution {
        ExecutionUseCaseImpl::new().execution(program, corpus, observer)
    }
}

impl PrepareUseCase for Execution {
    fn prepare(&self, request: PrepareRequest) -> Result<PreparedExecutionProgram, OperatingError> {
        self.0.prepare(request)
    }
}
