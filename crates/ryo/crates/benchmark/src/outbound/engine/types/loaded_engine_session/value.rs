//! Opaque loaded SDK state; callers can forward it but cannot replace its association.
use crate::outbound::engine::types::{EngineConfiguration, EngineRun};
use ryo::linter::{RelativePath, Rule, RuleId};
use ryo_benchmark_contracts::domain::{
    reports::errors::ReportsError, requests::types::WorkerRequest,
};
use std::collections::BTreeMap;
/// One loaded request, its exact rules, and its request-scoped effects.
///
/// ```compile_fail,E0451
/// use ryo_benchmark::outbound::engine::types::loaded_engine_session::value::LoadedEngineSession;
/// fn retarget<'a>(session: LoadedEngineSession<'a>, request: &'a ryo_benchmark_contracts::domain::requests::types::WorkerRequest) -> LoadedEngineSession<'a> {
///     LoadedEngineSession { request, ..session }
/// }
/// ```
/// ```compile_fail,E0624
/// use ryo_benchmark::outbound::engine::types::loaded_engine_session::value::LoadedEngineSession;
/// fn mutate(mut session: LoadedEngineSession<'_>) { session.rules_mut().clear(); }
/// ```
/// ```compile_fail,E0624
/// use ryo_benchmark::outbound::engine::types::loaded_engine_session::value::LoadedEngineSession;
/// fn forge() { let _constructor = LoadedEngineSession::new; }
/// ```
/// ```compile_fail,E0382
/// use ryo_benchmark::{application::worker_request::service::configure, outbound::engine::types::loaded_engine_session::value::LoadedEngineSession};
/// fn reuse(expected: &ryo_benchmark_contracts::domain::requests::types::WorkerRequest, loaded: LoadedEngineSession<'_>) { let _first = configure(expected, loaded); let _second = configure(expected, loaded); }
/// ```
/// ```compile_fail,E0624
/// use ryo_benchmark::outbound::engine::types::{loaded_engine_session::value::LoadedEngineSession, EngineConfiguration};
/// fn bypass(loaded: LoadedEngineSession<'_>, configuration: EngineConfiguration) { let _report = loaded.execute(configuration); }
/// ```
pub struct LoadedEngineSession<'a> {
    request: &'a WorkerRequest,
    rules: BTreeMap<RuleId, Rule>,
    frontend: Option<RuleId>,
    execution: Box<dyn EngineExecution + 'a>,
}
impl<'a> LoadedEngineSession<'a> {
    pub(crate) fn new(
        request: &'a WorkerRequest,
        rules: BTreeMap<RuleId, Rule>,
        frontend: Option<RuleId>,
        execution: Box<dyn EngineExecution + 'a>,
    ) -> Self {
        Self {
            request,
            rules,
            frontend,
            execution,
        }
    }
    pub(crate) fn request(&self) -> &'a WorkerRequest {
        self.request
    }
    pub(crate) fn rules_mut(&mut self) -> &mut BTreeMap<RuleId, Rule> {
        &mut self.rules
    }
    pub(crate) fn frontend_rule(&self) -> Option<&RuleId> {
        self.frontend.as_ref()
    }
    pub(crate) fn configuration_files(&self) -> Result<Vec<RelativePath>, ReportsError> {
        self.execution.configuration_files()
    }
    /// The time each of the loaded rules takes to compile ahead of its run, none from an engine that compiles inside the run.
    pub(crate) fn compile(
        &self,
        configuration: &EngineConfiguration,
    ) -> Option<std::collections::BTreeMap<RuleId, std::time::Duration>> {
        self.execution.compile(&self.rules, configuration)
    }
    pub(crate) fn execute(
        self,
        configuration: EngineConfiguration,
    ) -> Result<EngineRun, ReportsError> {
        self.execution.execute(self.rules, configuration)
    }
}

/// Crate-owned effects bound by the adapter when it issues the concrete loaded handle.
pub(crate) trait EngineExecution {
    /// Compile the configured rules ahead of the run; none for an engine that compiles inside it.
    fn compile(
        &self,
        _rules: &BTreeMap<RuleId, Rule>,
        _configuration: &EngineConfiguration,
    ) -> Option<std::collections::BTreeMap<RuleId, std::time::Duration>> {
        None
    }
    fn configuration_files(&self) -> Result<Vec<RelativePath>, ReportsError>;
    fn execute(
        self: Box<Self>,
        rules: BTreeMap<RuleId, Rule>,
        configuration: EngineConfiguration,
    ) -> Result<EngineRun, ReportsError>;
}
