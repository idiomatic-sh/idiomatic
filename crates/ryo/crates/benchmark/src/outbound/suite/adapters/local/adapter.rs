//! Validate pins, cases and rule coverage before running contestants.
use crate::domain::diagnostics::errors::Error;
use crate::domain::suite::types::Audit;
use crate::domain::suite::types::{Cases, Tool};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::{Component, Path};

pub fn audit(directory: &Path, tool: &Tool) -> Result<Audit, Error> {
    let defaults: BTreeSet<_> = tool.defaults.iter().cloned().collect();
    if defaults.is_empty() || defaults.len() != tool.defaults.len() {
        return Err(Error::Invalid(
            "empty or duplicated default rule inventory".into(),
        ));
    }
    let mut implemented = BTreeSet::new();
    for entry in std::fs::read_dir(directory.join("rules"))? {
        let entry = entry?;
        let rule = entry
            .file_name()
            .into_string()
            .map_err(|_| Error::Invalid("non-UTF8 rule id".into()))?;
        if !defaults.contains(&rule) {
            return Err(Error::Invalid(format!(
                "rule {rule} is not a shipped default"
            )));
        }
        let query = match std::fs::read_to_string(entry.path().join("query.ryoql")) {
            Ok(query) => Some(query),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        if query.as_ref().is_some_and(|query| query.trim().is_empty()) {
            return Err(Error::Invalid(format!("empty query for {rule}")));
        }
        let cases: Cases =
            super::schemas::cases(&std::fs::read_to_string(entry.path().join("cases.toml"))?)?;
        if !cases.cases.iter().any(|case| case.expected.is_empty())
            || !cases.cases.iter().any(|case| !case.expected.is_empty())
        {
            return Err(Error::Invalid(format!(
                "{rule} requires positive and negative cases"
            )));
        }
        for case in cases.cases {
            if case.files.is_empty() || case.name.trim().is_empty() {
                return Err(Error::Invalid(format!("invalid case for {rule}")));
            }
            for name in case.files.keys() {
                relative_path(name)?;
            }
            for expected in case.expected {
                let source = case
                    .files
                    .get(&expected.file)
                    .ok_or_else(|| Error::Invalid("expected finding outside case".into()))?;
                if expected.start > expected.end
                    || !source.is_char_boundary(expected.start)
                    || !source.is_char_boundary(expected.end)
                {
                    return Err(Error::Invalid(
                        "expected finding has invalid UTF-8 span".into(),
                    ));
                }
            }
        }
        if query.is_some() {
            implemented.insert(rule);
        }
    }
    Ok(Audit {
        missing: defaults.difference(&implemented).cloned().collect(),
        implemented: implemented.into_iter().collect(),
    })
}

pub fn require_complete(audit: &Audit) -> Result<(), Error> {
    if audit.missing.is_empty() {
        Ok(())
    } else {
        Err(Error::Invalid(format!(
            "default coverage incomplete: {} missing rules: {}",
            audit.missing.len(),
            audit.missing.join(", ")
        )))
    }
}

pub fn relative_path(path: &str) -> Result<(), Error> {
    if path.is_empty()
        || Path::new(path)
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(Error::Invalid(format!("unsafe corpus path {path:?}")));
    }
    Ok(())
}

pub fn verify_executable(path: &Path, tool: &Tool) -> Result<(), Error> {
    let digest = format!("{:x}", Sha256::digest(std::fs::read(path)?));
    if digest != tool.sha256 {
        return Err(Error::Invalid(
            "tool executable hash differs from pin".into(),
        ));
    }
    let output = std::process::Command::new(path).arg("--version").output()?;
    if !output.status.success()
        || String::from_utf8_lossy(&output.stdout).trim()
            != format!("{} {}", tool.tool, tool.version)
    {
        return Err(Error::Invalid("tool version differs from pin".into()));
    }
    Ok(())
}

#[derive(Debug)]
pub struct SuiteLocalAdapter;
impl crate::outbound::suite::interface::SuitePort for SuiteLocalAdapter {
    fn audit(&self, directory: &Path, tool: &Tool) -> Result<Audit, Error> {
        audit(directory, tool)
    }
}

use crate::domain::diagnostics::types::RunReport;
use crate::inbound::use_cases::compare::service::compare;
/// Check independently authored cases through both real process adapters.
pub fn verify_cases(
    reference: &dyn crate::outbound::linter::interface::LinterPort,
    execution: &dyn crate::outbound::execution::interface::ExecutionPort,
    candidate_executable: &std::path::Path,
    rules: &std::path::Path,
    scratch: &std::path::Path,
) -> Result<usize, crate::domain::diagnostics::errors::Error> {
    use crate::application::run::service::run;
    use crate::domain::{
        diagnostics::{errors::Error, types::Diagnostic},
        suite::types::{Cases, RunRequest},
    };
    use crate::outbound::linter::adapters::ryo::adapter::LinterRyoAdapter;
    let mut failures = Vec::new();
    let mut checked = 0;
    for entry in std::fs::read_dir(rules)? {
        let entry = entry?;
        let rule = entry
            .file_name()
            .into_string()
            .map_err(|_| Error::Invalid("non-UTF8 rule".into()))?;
        let query = match std::fs::read(entry.path().join("query.ryoql")) {
            Ok(query) => query,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                failures.push(format!("{rule}: query not implemented"));
                continue;
            }
            Err(error) => return Err(error.into()),
        };
        let cases: Cases =
            super::schemas::cases(&std::fs::read_to_string(entry.path().join("cases.toml"))?)?;
        let selected = scratch.join(&rule).join("rules");
        std::fs::create_dir_all(selected.join(&rule))?;
        std::fs::write(selected.join(&rule).join("query.ryoql"), query)?;
        let candidate = LinterRyoAdapter {
            executable: candidate_executable.to_owned(),
            rules: selected,
        };
        for (index, case) in cases.cases.into_iter().enumerate() {
            let root = scratch.join(&rule).join(index.to_string());
            std::fs::create_dir_all(&root)?;
            for (file, source) in &case.files {
                crate::outbound::suite::adapters::local::adapter::relative_path(file)?;
                let path = root.join(file);
                std::fs::create_dir_all(
                    path.parent()
                        .ok_or_else(|| Error::Invalid("missing case parent".into()))?,
                )?;
                std::fs::write(path, source)?;
            }
            let request = RunRequest {
                root,
                files: case.files.keys().cloned().collect(),
                cache: scratch.join("cache").join(&rule).join(index.to_string()),
            };
            let mut expected = RunReport {
                files: request.files.clone(),
                diagnostics: case
                    .expected
                    .into_iter()
                    .map(|finding| Diagnostic {
                        rule: rule.clone(),
                        file: finding.file,
                        start: finding.start,
                        end: finding.end,
                    })
                    .collect(),
            };
            expected.diagnostics.sort();
            for (name, tool) in [
                ("reference", reference),
                (
                    "candidate",
                    &candidate as &dyn crate::outbound::linter::interface::LinterPort,
                ),
            ] {
                match run(tool, execution, &request) {
                    Ok(mut result) => {
                        result
                            .report
                            .diagnostics
                            .retain(|finding| finding.rule == rule);
                        if let Err(difference) = compare(&expected, &result.report) {
                            failures.push(format!("{rule}/{} {name}: {difference:?}", case.name));
                        }
                    }
                    Err(error) => failures.push(format!("{rule}/{} {name}: {error}", case.name)),
                }
            }
            checked += 1;
        }
    }
    if failures.is_empty() {
        Ok(checked)
    } else {
        Err(Error::Invalid(format!(
            "{checked} cases checked; failures:\n{}",
            failures.join("\n")
        )))
    }
}
