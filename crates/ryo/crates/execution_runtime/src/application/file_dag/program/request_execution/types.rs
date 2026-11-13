//! One request execution: the state its files and project phase share.
use std::sync::Arc;

/// One request's execution, opened only by `begin`: its fields are sealed to the engine, so no
/// caller outside it reads or replaces the prepared program, the services, the frontier cache or
/// the digest port.
///
/// ```compile_fail,E0451
/// use ryo_execution_runtime::application::file_dag::program::request_execution::types::RequestExecution;
/// fn digest(execution: RequestExecution) {
///     let RequestExecution { digest, .. } = execution;
/// }
/// ```
pub struct RequestExecution {
    pub(crate) program:
        Arc<crate::application::file_dag::program::preparation::types::PreparedProgram>,
    pub(crate) services:
        Arc<crate::application::file_dag::program::facade_runtime::service::ProgramServices>,
    /// Prepared frontiers of this request, one acquisition per population.
    pub(crate) frontiers: crate::application::file_dag::frontiers::cache::service::FrontierCache,
    /// The prepared program's digest port, handed to each publication scope.
    pub(crate) digest: std::sync::Arc<dyn crate::outbound::digest::interface::DigestPort>,
}
