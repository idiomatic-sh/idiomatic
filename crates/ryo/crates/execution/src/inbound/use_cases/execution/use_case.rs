//! Thin public orchestration over the one private execution owner.
use super::interface::ExecutionUseCase;
use ryo_execution_runtime::outbound::observer::interface::ExecutionObserver;
use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use ryo_execution_runtime::application::file_dag::program::request_execution::service::begin;
use ryo_execution_runtime::application::file_dag::program::request_execution::types::RequestExecution;
use ryo_values::domain::files::types::CorpusInput;

/// Begins request executions; every kernel and service comes with the prepared program.
#[derive(Debug, Default)]
pub struct ExecutionUseCaseImpl;

impl ExecutionUseCaseImpl {
    /// A use case over the programs it is given.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl ExecutionUseCase for ExecutionUseCaseImpl {
    fn execution(
        &self,
        program: &PreparedExecutionProgram,
        corpus: &[CorpusInput],
        observer: Option<std::sync::Arc<dyn ExecutionObserver>>,
    ) -> RequestExecution {
        begin(program, corpus, observer)
    }
}
