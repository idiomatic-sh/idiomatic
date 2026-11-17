//! Execute the worker using typed engine and process-transport boundaries.
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
/// One child-process request, candidate evaluation and flushed response.
pub trait RunRyoUseCase {
    /// Receive, evaluate and publish one report in that order.
    ///
    /// # Errors
    /// Propagates receive, engine and response failures without publishing a clean result.
    fn run_ryo(&self) -> Result<(), Error>;
    /// Receive and configure the request as a run does, and compile its rules ahead of the run, answering the time each took;
    /// none when the request cannot be received or its engine compiles inside the run.
    fn compile(
        &self,
    ) -> Option<std::collections::BTreeMap<ryo::linter::RuleId, std::time::Duration>>;
}
