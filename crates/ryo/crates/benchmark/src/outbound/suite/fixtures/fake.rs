//! Empty in-memory suite preserves the distinction between defaults and implementations.
use crate::domain::suite::types::ArtifactDigest;
use crate::domain::{
    diagnostics::errors::DiagnosticsError as Error,
    suite::types::{Suite, Tool, audit::value::Audit},
};
use crate::outbound::suite::interface::SuitePort;
use crate::outbound::suite::types::{ArtifactPath, ArtifactSet};
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
    fn digest(&self, _: ArtifactPath<'_>) -> Result<ArtifactDigest, Error> {
        Err(std::io::Error::from(std::io::ErrorKind::NotFound).into())
    }
    fn cases(
        &self,
        _: ArtifactPath<'_>,
        _: ArtifactPath<'_>,
    ) -> Result<Vec<crate::domain::suite::types::PreparedCase>, Error> {
        if self.prepared.is_empty() {
            Err(Error::Invalid("no executable rule cases".into()))
        } else {
            Ok(self.prepared.clone())
        }
    }
    fn stage_projects(&self, _: ArtifactPath<'_>, _: ArtifactPath<'_>) -> Result<(), Error> {
        Err(std::io::Error::from(std::io::ErrorKind::NotFound).into())
    }
    fn stage(&self, _: ArtifactPath<'_>, _: ArtifactPath<'_>, _: &Audit) -> Result<(), Error> {
        Err(std::io::Error::from(std::io::ErrorKind::NotFound).into())
    }
    fn fingerprint(&self, _: ArtifactSet<'_>) -> Result<ArtifactDigest, Error> {
        Err(std::io::Error::from(std::io::ErrorKind::NotFound).into())
    }
    fn profile(&self, _: ArtifactPath<'_>) -> Result<Tool, Error> {
        self.profile
            .clone()
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound).into())
    }
    fn projects(&self, _: ArtifactPath<'_>) -> Result<Suite, Error> {
        self.projects
            .clone()
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound).into())
    }
    fn verify_tool(&self, _: ArtifactPath<'_>, tool: &Tool) -> Result<(), Error> {
        if self.profile.is_none() {
            return Err(std::io::Error::from(std::io::ErrorKind::NotFound).into());
        }
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

    fn audit(&self, _: ArtifactPath<'_>, tool: &Tool) -> Result<Audit, Error> {
        if tool.defaults.is_empty() {
            return Err(Error::Invalid(
                "empty or duplicated default rule inventory".into(),
            ));
        }
        let (implemented, missing) = tool
            .defaults
            .iter()
            .cloned()
            .partition(|rule| self.prepared.iter().any(|case| &case.rule == rule));
        Ok(Audit {
            implemented,
            missing,
        })
    }
}
#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
