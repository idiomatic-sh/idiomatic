//! Request-scoped access to the public SDK engine.
use super::types::loaded_engine_session::value::LoadedEngineSession;
use ryo_benchmark_contracts::domain::{
    reports::errors::ReportsError as Error, requests::types::WorkerRequest,
};
/// Open one SDK invocation; only the application can prepare the opaque loaded handle.
///
/// An external composition may forward the genuine handle without replacing its state.
/// ```no_run
/// use ryo_benchmark::outbound::engine::{interface::EnginePort, types::loaded_engine_session::value::LoadedEngineSession};
/// use ryo_benchmark_contracts::domain::{requests::types::WorkerRequest, reports::errors::ReportsError};
/// struct Forward<T>(T);
/// impl<T: EnginePort> EnginePort for Forward<T> {
///     fn load<'a>(&'a self, request: &'a WorkerRequest) -> Result<LoadedEngineSession<'a>, ReportsError> { self.0.load(request) }
/// }
/// ```
pub trait EnginePort {
    /// Validate physical inputs, establish diagnostics and load queries once.
    /// # Errors
    /// Refuses invalid physical context or query inputs before execution.
    fn load<'a>(&'a self, request: &'a WorkerRequest) -> Result<LoadedEngineSession<'a>, Error>;
    /// Compile the rules of `request` ahead of its first run, answering the time each took; an engine that compiles inside
    /// each run answers none.
    ///
    /// # Errors
    /// Refuses rules that cannot be loaded.
    fn compile(
        &self,
        _request: &WorkerRequest,
    ) -> Result<std::collections::BTreeMap<ryo::linter::RuleId, std::time::Duration>, Error> {
        Ok(std::collections::BTreeMap::new())
    }
}
