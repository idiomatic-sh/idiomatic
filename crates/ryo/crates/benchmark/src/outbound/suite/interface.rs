//! Load and validate the suite's external artifacts.
use crate::domain::suite::types::Audit;
use crate::domain::{diagnostics::errors::Error, suite::types::Tool};
use std::path::Path;
pub trait SuitePort {
    fn audit(&self, directory: &Path, tool: &Tool) -> Result<Audit, Error>;
}
