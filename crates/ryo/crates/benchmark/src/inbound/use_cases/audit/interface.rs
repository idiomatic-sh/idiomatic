//! Audit the exact shipped-default inventory.
use crate::domain::suite::types::Audit;
use crate::domain::{diagnostics::errors::Error, suite::types::Tool};
use std::path::Path;
pub trait AuditUseCase {
    fn audit(&self, directory: &Path, tool: &Tool) -> Result<Audit, Error>;
}
