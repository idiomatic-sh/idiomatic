//! Empty in-memory suite preserves the distinction between defaults and implementations.
use crate::domain::{
    diagnostics::errors::Error,
    suite::types::{Audit, Tool},
};
use crate::outbound::suite::interface::SuitePort;
use std::path::Path;
#[derive(Debug)]
pub struct SuiteFake;
impl SuitePort for SuiteFake {
    fn audit(&self, _: &Path, tool: &Tool) -> Result<Audit, Error> {
        if tool.defaults.is_empty() {
            return Err(Error::Invalid(
                "empty or duplicated default rule inventory".into(),
            ));
        }
        Ok(Audit {
            implemented: Vec::new(),
            missing: tool.defaults.clone(),
        })
    }
}
#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
