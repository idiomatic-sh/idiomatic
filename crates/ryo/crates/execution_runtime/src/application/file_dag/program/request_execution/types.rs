//! One request execution: the state its files and project phase share.
use std::sync::Arc;

/// One request's execution, opened only by `begin`: its fields are sealed to the engine, so no
/// caller outside it reads or replaces the prepared program, the services or the planned program.
///
/// ```compile_fail,E0451
/// use ryo_execution_runtime::application::file_dag::program::request_execution::types::RequestExecution;
/// fn pass(execution: RequestExecution) {
///     let RequestExecution { pass, .. } = execution;
/// }
/// ```
pub struct RequestExecution {
    pub(crate) program:
        Arc<crate::application::file_dag::program::preparation::types::PreparedProgram>,
    pub(crate) services: Arc<ProgramServices>,
    /// The program the file pass runs, planned when the program was prepared.
    pub(crate) pass: Arc<crate::application::file_dag::file_pass::types::PassProgram>,
}
impl std::fmt::Debug for RequestExecution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
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

/// The services one request's execution shares: the request's files and the observer.
pub(crate) struct ProgramServices {
    /// The request's files.
    pub request:
        Arc<crate::application::file_dag::program::request_identity::service::RequestIdentity>,
    /// The observer the request's caller asked to be told of the engine's work, if any.
    pub observer: Option<Arc<dyn crate::outbound::observer::interface::ObserverPort>>,
}
