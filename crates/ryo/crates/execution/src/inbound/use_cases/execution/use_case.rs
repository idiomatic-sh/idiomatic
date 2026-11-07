//! Thin public orchestration over the one private execution owner.
use crate::inbound::use_cases::execution::interface::ExecutionUseCase;
use crate::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use crate::application::file_dag::program::request_execution::service::begin;
use crate::application::file_dag::program::request_execution::types::RequestExecution;
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
    fn execution(&self, program: &PreparedExecutionProgram, corpus: &[CorpusInput]) -> RequestExecution {
        begin(program, corpus)
    }
}
