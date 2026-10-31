//! Empty in-memory suite preserves the distinction between defaults and implementations.
use crate::domain::{
    diagnostics::errors::Error,
    suite::types::{Audit, Suite, Tool},
};
use crate::outbound::suite::interface::SuitePort;
use std::path::Path;
#[derive(Clone, Debug, Default)]
/// Deterministic suite substitute for orchestration and contract tests.
pub struct SuiteFake {
    /// The profile available from this deterministic artifact source.
    pub profile: Option<Tool>,
    /// The projects available from this deterministic artifact source.
    pub projects: Option<Suite>,
    /// Independent cases already materialized by the fixture.
    pub prepared: Vec<crate::domain::suite::types::PreparedCase>,
}
impl SuitePort for SuiteFake {
    fn cases(
        &self,
        _: &Path,
        _: &Path,
    ) -> Result<Vec<crate::domain::suite::types::PreparedCase>, Error> {
        if self.prepared.is_empty() {
            Err(Error::Invalid("no executable rule cases".into()))
        } else {
            Ok(self.prepared.clone())
        }
    }
    fn stage(&self, _: &Path, _: &Path, _: &Audit) -> Result<(), Error> {
        Err(Error::Invalid(
            "fixture cannot materialize external queries".into(),
        ))
    }
    fn fingerprint(&self, _: &[std::path::PathBuf]) -> Result<String, Error> {
        Err(Error::Invalid(
            "fixture has no immutable artifact snapshot".into(),
        ))
    }
    fn profile(&self, _: &Path) -> Result<Tool, Error> {
        self.profile
            .clone()
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound).into())
    }
    fn projects(&self, _: &Path) -> Result<Suite, Error> {
        self.projects
            .clone()
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound).into())
    }
    fn verify_tool(&self, _: &Path, tool: &Tool) -> Result<(), Error> {
        match &self.profile {
            Some(expected)
                if expected.tool == tool.tool
                    && expected.version == tool.version
                    && expected.sha256 == tool.sha256
                    && expected.defaults == tool.defaults =>
            {
                Ok(())
            }
            _ => Err(Error::Invalid(
                "tool does not match configured fixture profile".into(),
            )),
        }
    }

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
