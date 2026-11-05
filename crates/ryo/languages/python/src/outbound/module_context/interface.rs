//! One explicit project lookup boundary, preserving Disk versus Given semantics.
use crate::domain::module_context::types::ModuleContexts;
use ryo::languages::{ProjectError, ProjectRequest};
/// Establish package context from the request's permitted repository input.
pub(crate) trait ModuleContextPort: Send + Sync {
    /// Return known module identities and all examined marker dependencies, or an honest lookup failure.
    fn resolve(&self, request: &ProjectRequest) -> Result<ModuleContexts, ProjectError>;
}
