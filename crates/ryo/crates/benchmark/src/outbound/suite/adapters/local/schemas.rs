//! Private TOML DTOs map to canonical benchmark values at this boundary.
use crate::domain::{diagnostics::errors::DiagnosticsError as Error, suite::types};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolSchema {
    tool: String,
    version: String,
    sha256: String,
    defaults: Vec<String>,
    #[serde(default)]
    deferred: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pending: BTreeMap<String, Vec<String>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SuiteSchema {
    language: String,
    projects: Vec<ProjectSchema>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectSchema {
    name: String,
    repository: String,
    revision: String,
    patch: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CasesSchema {
    cases: Vec<CaseSchema>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CaseSchema {
    name: String,
    files: BTreeMap<String, String>,
    expected: Vec<ExpectedSchema>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedSchema {
    file: String,
    start: usize,
    end: usize,
}

pub(super) fn tool(text: &str) -> Result<types::Tool, Error> {
    toml::from_str::<ToolSchema>(text)?.try_into()
}
impl TryFrom<ToolSchema> for types::Tool {
    type Error = Error;
    fn try_from(wire: ToolSchema) -> Result<Self, Error> {
        Ok(Self {
            tool: wire.tool.into(),
            version: wire.version.into(),
            sha256: wire.sha256.into(),
            defaults: wire
                .defaults
                .iter()
                .map(|value| rule(value))
                .collect::<Result<_, _>>()?,
            deferred: dispositions(wire.deferred)?,
            pending: dispositions(wire.pending)?,
        })
    }
}
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
pub(super) fn rule(value: &str) -> Result<ryo::linter::RuleId, Error> {
    ryo::linter::RuleId::try_from(value).map_err(|error| Error::Invalid(error.to_string()))
}
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
pub(super) fn cases(text: &str) -> Result<types::Cases, Error> {
    Ok(toml::from_str::<CasesSchema>(text)?.into())
}
impl From<CasesSchema> for types::Cases {
    fn from(wire: CasesSchema) -> Self {
        Self {
            cases: wire
                .cases
                .into_iter()
                .map(|case| types::Case {
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
                        .map(|expected| types::Expected {
                            file: ryo::linter::RelativePath(expected.file.into_boxed_str()),
                            start: crate::domain::diagnostics::types::SourceByteOffset(
                                expected.start,
                            ),
                            end: crate::domain::diagnostics::types::SourceByteOffset(expected.end),
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}
#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
