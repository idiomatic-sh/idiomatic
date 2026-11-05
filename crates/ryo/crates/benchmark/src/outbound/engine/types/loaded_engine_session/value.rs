//! Opaque loaded SDK state; callers can forward it but cannot replace its association.
use crate::outbound::engine::types::EngineConfiguration;
use ryo::linter::{RelativePath, Rule, RuleId};
use ryo_benchmark_contracts::domain::{
    reports::{errors::ReportsError, types::RunReport},
    requests::types::WorkerRequest,
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
    pub(crate) fn execute(
        self,
        configuration: EngineConfiguration,
    ) -> Result<RunReport, ReportsError> {
        self.execution.execute(self.rules, configuration)
    }
}

/// Crate-owned effects bound by the adapter when it issues the concrete loaded handle.
pub(crate) trait EngineExecution {
    fn configuration_files(&self) -> Result<Vec<RelativePath>, ReportsError>;
    fn execute(
        self: Box<Self>,
        rules: BTreeMap<RuleId, Rule>,
        configuration: EngineConfiguration,
    ) -> Result<RunReport, ReportsError>;
}
