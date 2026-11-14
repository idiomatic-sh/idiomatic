//! One request execution: the state its files and project phase share.
use ryo_execution_domain::domain::operating::errors::OperatingError;
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
    pub(crate) digest: Arc<dyn crate::outbound::digest::interface::DigestPort>,
    /// The program the file pass runs, or why it could not be made, when the differential asks for it. Branch only: until the
    /// switch.
    pub(crate) pass: Option<
        Result<crate::application::file_dag::file_pass::types::PassProgram, OperatingError>,
    >,
}

/// The arenas the cross-file observations publish their stored facts into, kept by the caller across requests.
#[derive(Debug, Default)]
pub struct ObservationArenas(pub Vec<ryo_values::domain::stored_facts::types::ArenaBytes>);

impl std::ops::Deref for ObservationArenas {
    type Target = Vec<ryo_values::domain::stored_facts::types::ArenaBytes>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for ObservationArenas {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
