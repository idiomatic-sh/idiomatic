//! Completed semantic construction data awaiting the common acquisition decision.
use crate::application::file_dag::derived::owner::acquisition::population::types::Construction;
use crate::application::file_dag::derived::owner::acquisition::semantics::storage::DerivedStorage;
use std::sync::Arc;
pub(in crate::application::file_dag::derived::owner::acquisition::graph) struct BuiltGraph {
    pub storage: Arc<DerivedStorage>,
    pub construction: Construction,
}
pub(in crate::application::file_dag::derived::owner::acquisition::graph) struct BuiltDomain {
    pub storage: Arc<DerivedStorage>,
    pub construction: Construction,
}
