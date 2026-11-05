//! A checked transition exposes only consuming execution.
/// The configured transition exposes only consuming execution, never mutable rules.
///
/// ```no_run
/// use ryo_benchmark::outbound::engine::types::configured_engine_session::value::ConfiguredEngineSession;
/// fn execute(configured: ConfiguredEngineSession<'_>) { let _result = configured.execute(); }
/// ```
///
/// ```compile_fail,E0599
/// use ryo_benchmark::outbound::engine::types::configured_engine_session::value::ConfiguredEngineSession;
/// fn mutate(mut configured: ConfiguredEngineSession<'_>) {
///     configured.request_and_rules();
/// }
/// ```
/// ```compile_fail,E0451
/// use ryo_benchmark::outbound::engine::types::{configured_engine_session::value::ConfiguredEngineSession, loaded_engine_session::value::LoadedEngineSession, EngineConfiguration};
/// fn bypass<'a>(session: LoadedEngineSession<'a>, context: EngineConfiguration) -> ConfiguredEngineSession<'a> {
///     ConfiguredEngineSession { session, context }
/// }
/// ```
pub struct ConfiguredEngineSession<'a> {
    session: crate::outbound::engine::types::loaded_engine_session::value::LoadedEngineSession<'a>,
    context: crate::outbound::engine::types::EngineConfiguration,
}
impl<'a> ConfiguredEngineSession<'a> {
    pub(crate) fn new(
        session: crate::outbound::engine::types::loaded_engine_session::value::LoadedEngineSession<
            'a,
        >,
        context: crate::outbound::engine::types::EngineConfiguration,
    ) -> Self {
        Self { session, context }
    }
    /// Consume the same configured session and flush its diagnostics on completion.
    pub fn execute(
        self,
    ) -> Result<
        ryo_benchmark_contracts::domain::reports::types::RunReport,
        ryo_benchmark_contracts::domain::reports::errors::ReportsError,
    > {
        self.session.execute(self.context)
    }
}
