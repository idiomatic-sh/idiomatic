//! Private TOML DTOs map to canonical benchmark values at this boundary.
use crate::domain::{diagnostics::errors::Error, suite::types};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolWire {
    tool: String,
    version: String,
    sha256: String,
    defaults: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SuiteWire {
    language: String,
    projects: Vec<ProjectWire>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectWire {
    name: String,
    repository: String,
    revision: String,
    patch: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CasesWire {
    cases: Vec<CaseWire>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CaseWire {
    name: String,
    files: BTreeMap<String, String>,
    expected: Vec<ExpectedWire>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedWire {
    file: String,
    start: usize,
    end: usize,
}

pub fn tool(text: &str) -> Result<types::Tool, Error> {
    let wire: ToolWire = toml::from_str(text)?;
    Ok(types::Tool {
        tool: wire.tool,
        version: wire.version,
        sha256: wire.sha256,
        defaults: wire.defaults,
    })
}
pub fn suite(text: &str) -> Result<types::Suite, Error> {
    let wire: SuiteWire = toml::from_str(text)?;
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
pub fn cases(text: &str) -> Result<types::Cases, Error> {
    let wire: CasesWire = toml::from_str(text)?;
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
