//! Shared language corpus manifest.
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types;
use serde::Deserialize;

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
/// Shared language corpus manifest.
struct SuiteSchema {
    /// Language identity for the corpus.
    language: String,
    /// Pinned repositories included in the workload.
    projects: Vec<ProjectSchema>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
/// One reproducible repository workload.
struct ProjectSchema {
    /// Stable project identity.
    name: String,
    /// Repository URL used to obtain the source.
    repository: String,
    /// Exact source revision to check out.
    revision: String,
    /// Manifest-relative edit patch for incremental trials.
    patch: String,
}
/// Decode the shared language corpus manifest.
pub(in super::super) fn suite(text: &str) -> Result<types::Suite, Error> {
    let wire: SuiteSchema = toml::from_str(text)?;
    if wire.language.trim().is_empty() || wire.projects.is_empty() {
        return Err(Error::Invalid(
            "corpus must name a language and at least one project".into(),
        ));
    }
    {
        let mut names = std::collections::BTreeSet::new();
        for project in &wire.projects {
            if project.name.trim().is_empty() || !names.insert(project.name.as_str()) {
                return Err(Error::Invalid(
                    "corpus project names must be nonempty and unique".into(),
                ));
            }
        }
    }
    Ok(wire.into())
}
impl From<SuiteSchema> for types::Suite {
    fn from(wire: SuiteSchema) -> Self {
        Self {
            language: wire.language.into(),
            projects: wire.projects.into_iter().map(Into::into).collect(),
        }
    }
}
impl From<ProjectSchema> for types::Project {
    fn from(project: ProjectSchema) -> Self {
        Self {
            name: project.name.into(),
            repository: project.repository.into(),
            revision: project.revision.into(),
            patch: project.patch.into(),
        }
    }
}
