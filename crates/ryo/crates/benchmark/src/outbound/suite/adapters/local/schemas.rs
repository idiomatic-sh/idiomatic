//! Private TOML DTOs map to canonical benchmark values at this boundary.
use crate::domain::{diagnostics::errors::Error, suite::types};
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
    let wire: ToolSchema = toml::from_str(text)?;
    Ok(types::Tool {
        tool: wire.tool,
        version: wire.version,
        sha256: wire.sha256,
        defaults: wire.defaults,
        deferred: dispositions(wire.deferred)?,
        pending: dispositions(wire.pending)?,
    })
}
fn dispositions(groups: BTreeMap<String, Vec<String>>) -> Result<BTreeMap<String, String>, Error> {
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
        for rule in members {
            if rules.insert(rule, capability.clone()).is_some() {
                return Err(Error::Invalid(
                    "rule appears in multiple capability groups".into(),
                ));
            }
        }
    }
    Ok(rules)
}

pub(super) fn suite(text: &str) -> Result<types::Suite, Error> {
    let wire: SuiteSchema = toml::from_str(text)?;
    Ok(types::Suite {
        language: wire.language,
        projects: wire
            .projects
            .into_iter()
            .map(|project| types::Project {
                name: project.name,
                repository: project.repository,
                revision: project.revision,
                patch: project.patch,
            })
            .collect(),
    })
}
pub(super) fn cases(text: &str) -> Result<types::Cases, Error> {
    let wire: CasesSchema = toml::from_str(text)?;
    Ok(types::Cases {
        cases: wire
            .cases
            .into_iter()
            .map(|case| types::Case {
                name: case.name,
                files: case.files,
                expected: case
                    .expected
                    .into_iter()
                    .map(|expected| types::Expected {
                        file: expected.file,
                        start: expected.start,
                        end: expected.end,
                    })
                    .collect(),
            })
            .collect(),
    })
}
