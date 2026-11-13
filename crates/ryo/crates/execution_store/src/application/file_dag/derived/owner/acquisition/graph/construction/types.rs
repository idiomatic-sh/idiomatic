//! Completed semantic construction data awaiting the common acquisition decision.
use crate::application::file_dag::derived::owner::acquisition::population::types::Construction;
use crate::application::file_dag::derived::owner::acquisition::semantics::storage::service::DerivedStorage;
use std::sync::Arc;
pub(crate) struct BuiltGraph {
    pub storage: Arc<DerivedStorage>,
    pub construction: Construction,
}
pub(crate) struct BuiltDomain {
    pub storage: Arc<DerivedStorage>,
    pub construction: Construction,
}
