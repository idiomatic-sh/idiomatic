//! Private TOML DTOs map to canonical benchmark values at this boundary.
use crate::domain::{diagnostics::errors::DiagnosticsError as Error, suite::types};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
/// Pinned reference profile and explicit default-rule accounting.
struct ToolSchema {
    /// Reference tool name.
    tool: String,
    /// Exact expected version output.
    version: String,
    /// Trusted platform-specific SHA256 digests of the executable bytes.
    sha256: Vec<String>,
    /// Resolved default rule inventory for the pinned tool.
    defaults: Vec<String>,
    #[serde(default)]
    /// Capability groups whose rules remain blocked by documented limitations.
    deferred: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    /// Capability groups whose rules remain implementation work.
    pending: BTreeMap<String, Vec<String>>,
    /// Explicit logical query-module IDs mapped to profile-relative source paths.
    #[serde(default, rename = "query-modules")]
    query_modules: BTreeMap<String, String>,
    #[serde(default, rename = "parser-configuration")]
    /// Language parser configuration shared by both contestants.
    parser_configuration: Option<String>,
}
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
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
/// Independent rule examples and expected findings.
struct CasesSchema {
    /// Ordered set of executable examples.
    cases: Vec<CaseSchema>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
/// One independent source example.
struct CaseSchema {
    /// Explicit diagnostic scope; existing cases assert their enclosing rule.
    #[serde(default)]
    comparison: ComparisonSchema,
    /// Human-readable case identity.
    name: String,
    /// Project-relative source paths mapped to complete source text.
    files: BTreeMap<String, String>,
    /// Expected diagnostic ranges, preserving multiplicity.
    expected: Vec<ExpectedSchema>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
/// Expected finding in one authored case.
struct ExpectedSchema {
    /// Explicit diagnostic identity, required in full-report mode.
    #[serde(default)]
    rule: Option<String>,
    /// Project-relative source path.
    file: String,
    /// Inclusive zero-based UTF-8 byte offset.
    start: usize,
    /// Exclusive zero-based UTF-8 byte offset.
    end: usize,
}

/// Serialized fixture comparison scope.
#[derive(Default, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ComparisonSchema {
    /// Compare the enclosing rule only.
    #[default]
    Rule,
    /// Compare all reported diagnostics.
    Full,
}

/// Decode and validate a pinned reference profile.
pub(super) fn tool(text: &str) -> Result<types::Tool, Error> {
    toml::from_str::<ToolSchema>(text)?.try_into()
}
impl TryFrom<ToolSchema> for types::Tool {
    type Error = Error;
    fn try_from(wire: ToolSchema) -> Result<Self, Error> {
        if wire.sha256.is_empty()
            || wire
                .sha256
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != wire.sha256.len()
        {
            return Err(Error::Invalid(
                "executable pins must be nonempty and unique".into(),
            ));
        }
        let mut query_modules = BTreeMap::new();
        for (id, path) in wire.query_modules {
            let _module = ryo::query::QueryModuleSource::try_new(&id, "module source")
                .map_err(|error| Error::Invalid(error.to_string()))?;
            let path = std::path::PathBuf::from(path);
            crate::domain::suite::service::relative_path(&path)?;
            if query_modules.insert(id, path).is_some() {
                return Err(Error::Invalid("duplicate query module ID".into()));
            }
        }
        if wire
            .parser_configuration
            .as_ref()
            .is_some_and(|configuration| configuration.trim().is_empty())
        {
            return Err(Error::Invalid(
                "parser configuration must not be blank".into(),
            ));
        }
        Ok(Self {
            tool: wire.tool.into(),
            version: wire.version.into(),
            sha256: wire.sha256.into_iter().map(Into::into).collect(),
            defaults: wire
                .defaults
                .iter()
                .map(|value| rule(value))
                .collect::<Result<_, _>>()?,
            deferred: dispositions(wire.deferred)?,
            pending: dispositions(wire.pending)?,
            query_modules,
            parser_configuration: wire.parser_configuration,
        })
    }
}
/// Expand grouped capabilities while rejecting duplicate rule membership.
fn dispositions(
    groups: BTreeMap<String, Vec<String>>,
) -> Result<BTreeMap<ryo::linter::RuleId, types::CapabilityName>, Error> {
    let mut rules = BTreeMap::new();
    for (capability, members) in groups {
        if capability.is_empty()
            || !capability
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
            || members.is_empty()
        {
            return Err(Error::Invalid(
                "capability groups require a stable name and rules".into(),
            ));
        }
        for value in members {
            if rules
                .insert(rule(&value)?, types::CapabilityName(capability.clone()))
                .is_some()
            {
                return Err(Error::Invalid(
                    "rule appears in multiple capability groups".into(),
                ));
            }
        }
    }
    Ok(rules)
}
/// Validate a rule identifier before it enters the benchmark domain.
pub(super) fn rule(value: &str) -> Result<ryo::linter::RuleId, Error> {
    ryo::linter::RuleId::try_from(value).map_err(|error| Error::Invalid(error.to_string()))
}
/// Decode the shared language corpus manifest.
pub(super) fn suite(text: &str) -> Result<types::Suite, Error> {
    Ok(toml::from_str::<SuiteSchema>(text)?.into())
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
/// Decode independent sources and expected byte ranges.
pub(super) fn cases(text: &str) -> Result<types::Cases, Error> {
    toml::from_str::<CasesSchema>(text)?.try_into()
}
impl TryFrom<CasesSchema> for types::Cases {
    type Error = Error;
    fn try_from(wire: CasesSchema) -> Result<Self, Error> {
        Ok(Self {
            cases: wire
                .cases
                .into_iter()
                .map(|case| {
                    Ok(types::Case {
                        comparison: match case.comparison {
                            ComparisonSchema::Rule => types::CaseComparison::Rule,
                            ComparisonSchema::Full => types::CaseComparison::Full,
                        },
                        name: case.name.into(),
                        files: case
                            .files
                            .into_iter()
                            .map(|(path, text)| {
                                (
                                    ryo::linter::RelativePath(path.into_boxed_str()),
                                    ryo::linter::SourceText(text.into_boxed_str()),
                                )
                            })
                            .collect(),
                        expected: case
                            .expected
                            .into_iter()
                            .map(|expected| {
                                Ok(types::Expected {
                                    identity: match expected.rule {
                                        Some(value) => types::ExpectedRule::Explicit(rule(&value)?),
                                        None => types::ExpectedRule::Parent,
                                    },
                                    file: ryo::linter::RelativePath(expected.file.into_boxed_str()),
                                    start: crate::domain::diagnostics::types::SourceByteOffset(
                                        expected.start,
                                    ),
                                    end: crate::domain::diagnostics::types::SourceByteOffset(
                                        expected.end,
                                    ),
                                })
                            })
                            .collect::<Result<_, Error>>()?,
                    })
                })
                .collect::<Result<_, Error>>()?,
        })
    }
}
#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
