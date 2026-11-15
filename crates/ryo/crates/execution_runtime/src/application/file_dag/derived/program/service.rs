//! The `program` application operation: the request's derived access, held for a prepared program.
use ryo_execution_store::application::file_dag::derived::owner::service::DerivedAccess;
use ryo_execution_store::application::file_dag::derived::request::service::RequestViews;

/// The request's derived views as a prepared program uses them: it binds each operator's derived views once, and opens
/// the relation owner of every file and project scope. It lends no access and acquires no view.
#[derive(Clone, Debug)]
pub(crate) struct ProgramDerived(DerivedAccess);

impl<T: Into<DerivedAccess>> From<T> for ProgramDerived {
    fn from(access: T) -> Self {
        Self(access.into())
    }
}

impl ProgramDerived {
    /// The views that bind the derived slots of the request's operators, each prepared once for the request.
    pub(crate) fn request_views(&self) -> RequestViews {
        RequestViews::new(self.0.clone())
    }
}
