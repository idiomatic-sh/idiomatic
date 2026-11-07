//! One request execution: the state its files and project phase share.
use std::sync::Arc;
pub struct RequestExecution {
    pub(crate) program:
        Arc<crate::application::file_dag::program::preparation::types::PreparedProgram>,
    pub(crate) services:
        Arc<crate::application::file_dag::program::facade_runtime::types::ProgramServices>,
    /// Prepared frontiers of this request, one acquisition per population.
    pub(crate) frontiers: crate::application::file_dag::frontiers::types::FrontierCache,
    /// The prepared program's digest port, handed to each publication scope.
    pub(crate) digest: std::sync::Arc<dyn crate::outbound::digest::interface::DigestPort>,
}

