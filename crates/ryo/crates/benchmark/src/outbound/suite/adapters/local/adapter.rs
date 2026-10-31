//! Validate pins, cases and rule coverage before running contestants.
use crate::domain::diagnostics::errors::Error;
use crate::domain::suite::service::{relative_path, validate_case_sources, validate_cases};
use crate::domain::suite::types::audit::value::Audit;
use crate::domain::suite::types::{Cases, Tool};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::Path;

fn audit(directory: &Path, tool: &Tool) -> Result<Audit, Error> {
    let defaults: BTreeSet<_> = tool.defaults.iter().cloned().collect();
    if defaults.is_empty() || defaults.len() != tool.defaults.len() {
        return Err(Error::Invalid(
            "empty or duplicated default rule inventory".into(),
        ));
    }
    for (rule, issue) in tool.deferred.iter().chain(&tool.pending) {
        if !defaults.contains(rule)
            || issue.is_empty()
            || (tool.deferred.contains_key(rule) && tool.pending.contains_key(rule))
        {
            return Err(Error::Invalid(format!(
                "invalid or overlapping default disposition: {rule} -> {issue}"
            )));
        }
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
        validate_case_sources(&cases)?;
        if query.is_some() {
            validate_cases(&cases)?;
            if tool.deferred.contains_key(&rule) || tool.pending.contains_key(&rule) {
                return Err(Error::Invalid(format!(
                    "authored rule still has a deferred or pending disposition: {rule}"
                )));
            }
            let _ = implemented.insert(rule);
        }
    }
    Ok(Audit {
        missing: defaults.difference(&implemented).cloned().collect(),
        implemented: implemented.into_iter().collect(),
    })
}

fn verify_executable(path: &Path, tool: &Tool) -> Result<(), Error> {
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
pub(crate) struct SuiteLocalAdapter;
impl crate::outbound::suite::interface::SuitePort for SuiteLocalAdapter {
    fn cases(
        &self,
        rules: &Path,
        scratch: &Path,
    ) -> Result<Vec<crate::domain::suite::types::PreparedCase>, Error> {
        prepare_cases(rules, scratch)
    }
    fn stage_projects(&self, source: &Path, destination: &Path) -> Result<(), Error> {
        let source = source.canonicalize()?;
        let bytes = std::fs::read(source.join("benchmark.toml"))?;
        let manifest = super::schemas::suite(
            std::str::from_utf8(&bytes).map_err(|error| Error::Invalid(error.to_string()))?,
        )?;
        std::fs::create_dir(destination)?;
        std::fs::write(destination.join("benchmark.toml"), bytes)?;
        for project in manifest.projects {
            relative_path(&project.patch)?;
            let target = destination.join(&project.patch);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let patch = source.join(project.patch).canonicalize()?;
            if !patch.starts_with(&source) {
                return Err(Error::Invalid("patch escapes the shared suite directory".into()));
            }
            std::fs::write(target, std::fs::read(patch)?)?;
        }
        Ok(())
    }
    fn stage(&self, directory: &Path, destination: &Path, audit: &Audit) -> Result<(), Error> {
        std::fs::create_dir(destination)?;
        std::fs::write(
            destination.join("benchmark.toml"),
            std::fs::read(directory.join("benchmark.toml"))?,
        )?;
        std::fs::create_dir(destination.join("rules"))?;
        for rule in &audit.implemented {
            relative_path(rule)?;
            let target = destination.join("rules").join(rule);
            std::fs::create_dir(&target)?;
            let bytes = std::fs::read(directory.join("rules").join(rule).join("query.ryoql"))?;
            std::fs::write(target.join("query.ryoql"), bytes)?;
            std::fs::write(
                target.join("cases.toml"),
                std::fs::read(directory.join("rules").join(rule).join("cases.toml"))?,
            )?;
        }
        Ok(())
    }
    fn fingerprint(&self, paths: &[std::path::PathBuf]) -> Result<String, Error> {
        let mut hasher = Sha256::new();
        for path in paths {
            fingerprint_path(path, &mut hasher)?;
        }
        Ok(format!("{:x}", hasher.finalize()))
    }
    fn profile(&self, directory: &Path) -> Result<Tool, Error> {
        super::schemas::tool(&std::fs::read_to_string(directory.join("benchmark.toml"))?)
    }
    fn projects(&self, directory: &Path) -> Result<crate::domain::suite::types::Suite, Error> {
        super::schemas::suite(&std::fs::read_to_string(directory.join("benchmark.toml"))?)
    }
    fn verify_tool(&self, executable: &Path, tool: &Tool) -> Result<(), Error> {
        verify_executable(executable, tool)
    }

    fn audit(&self, directory: &Path, tool: &Tool) -> Result<Audit, Error> {
        audit(directory, tool)
    }
}

fn fingerprint_path(path: &Path, hasher: &mut Sha256) -> Result<(), Error> {
    let metadata = std::fs::symlink_metadata(path)?;
    let name = path
        .to_str()
        .ok_or_else(|| Error::Invalid("non-UTF8 evidence path".into()))?;
    hasher.update(name.len().to_le_bytes());
    hasher.update(name.as_bytes());
    if metadata.is_file() {
        let bytes = std::fs::read(path)?;
        hasher.update(bytes.len().to_le_bytes());
        hasher.update(bytes);
    } else if metadata.is_dir() {
        let mut children = std::fs::read_dir(path)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()?;
        children.sort();
        for child in children {
            fingerprint_path(&child, hasher)?;
        }
    } else {
        return Err(Error::Invalid(
            "evidence paths must be regular files or directories".into(),
        ));
    }
    Ok(())
}

use crate::domain::diagnostics::types::RunReport;
fn prepare_cases(
    rules: &Path,
    scratch: &Path,
) -> Result<Vec<crate::domain::suite::types::PreparedCase>, Error> {
    use crate::domain::{
        diagnostics::types::Diagnostic,
        suite::types::{PreparedCase, RunRequest},
    };
    let mut prepared = Vec::new();
    for entry in std::fs::read_dir(rules)? {
        let entry = entry?;
        let rule = entry
            .file_name()
            .into_string()
            .map_err(|_| Error::Invalid("non-UTF8 rule".into()))?;
        let cases: Cases =
            super::schemas::cases(&std::fs::read_to_string(entry.path().join("cases.toml"))?)?;
        validate_case_sources(&cases)?;
        match std::fs::read_to_string(entry.path().join("query.ryoql")) {
            Ok(query) if !query.trim().is_empty() => {}
            Ok(_) => return Err(Error::Invalid(format!("empty query for {rule}"))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        }
        validate_cases(&cases)?;
        for (index, case) in cases.cases.into_iter().enumerate() {
            let root = scratch.join(&rule).join(index.to_string());
            std::fs::create_dir_all(&root)?;
            for (file, source) in &case.files {
                relative_path(file)?;
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
            prepared.push(PreparedCase {
                rule: rule.clone(),
                name: case.name,
                request,
                expected,
            });
        }
    }
    if prepared.is_empty() {
        return Err(Error::Invalid("no executable rule cases".into()));
    }
    Ok(prepared)
}
