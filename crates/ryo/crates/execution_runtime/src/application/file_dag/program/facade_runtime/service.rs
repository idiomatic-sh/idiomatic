//! The services one request's execution shares: the request's files and the observer.
use crate::application::file_dag::program::request_identity::service::RequestIdentity;
use std::sync::Arc;

pub(crate) struct ProgramServices {
    pub(super) request: Arc<RequestIdentity>,
    /// The observer the request's caller asked to be told of the engine's work, if any.
    pub(super) observer: Option<Arc<dyn crate::outbound::observer::interface::ObserverPort>>,
}

impl ProgramServices {
    pub(crate) fn new(corpus: &[ryo_values::domain::files::types::CorpusInput]) -> Self {
        Self {
            request: Arc::new(RequestIdentity::new(corpus)),
            observer: None,
        }
    }
    /// The same services, telling `observer` of the engine's work.
    pub(crate) fn observed(
        mut self,
        observer: Option<Arc<dyn crate::outbound::observer::interface::ObserverPort>>,
    ) -> Self {
        self.observer = observer;
        self
    }
    /// The observer the request's caller asked to be told of the engine's work, if any.
    pub(crate) fn observer(
        &self,
    ) -> Option<&dyn crate::outbound::observer::interface::ObserverPort> {
        self.observer.as_deref()
    }
    pub(crate) fn request(&self) -> &Arc<RequestIdentity> {
        &self.request
    }
}
