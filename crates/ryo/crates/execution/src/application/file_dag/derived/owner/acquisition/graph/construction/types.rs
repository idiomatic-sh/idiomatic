//! Completed semantic construction data awaiting the common acquisition decision.
use crate::application::file_dag::derived::owner::acquisition::semantics::service::DerivedStorage;
use std::sync::Arc;
pub(in crate::application::file_dag::derived::owner::acquisition::graph) struct BuiltGraph {
    pub storage: Arc<DerivedStorage>,
    pub built: bool,
}
pub(in crate::application::file_dag::derived::owner::acquisition::graph) struct BuiltDomain {
    pub storage: Arc<DerivedStorage>,
    pub built: bool,
}
