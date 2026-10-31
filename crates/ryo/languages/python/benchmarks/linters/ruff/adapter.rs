//! Ruff owns its invocation and JSON decoding; shared execution owns timing.
use super::schemas::{Finding, offset};
use ryo_benchmark::domain::measurements::types::{Invocation, ProcessOutput};
use ryo_benchmark::domain::{
    diagnostics::{
        errors::Error,
        types::{Diagnostic, RunReport},
    },
    suite::types::RunRequest,
};
use ryo_benchmark::outbound::linter::interface::LinterPort;
use std::collections::BTreeSet;
use std::path::PathBuf;

#[derive(Debug)]
pub(crate) struct Ruff {
    pub executable: PathBuf,
    pub defaults: BTreeSet<String>,
}
impl LinterPort for Ruff {
    fn prepare(&self, request: &RunRequest) -> Result<Invocation, Error> {
        if request.files.is_empty() {
            return Err(Error::Invalid(
                "empty input list would make Ruff walk the directory".into(),
            ));
        }
        let mut arguments = vec![
            "check".into(),
            "--isolated".into(),
            "--output-format".into(),
            "json".into(),
            "--cache-dir".into(),
            request.cache.join("ruff").into_os_string(),
            "--".into(),
        ];
        for file in &request.files {
            ryo_benchmark::outbound::suite::adapters::local::adapter::relative_path(file)?;
            arguments.push(file.into());
        }
        Ok(Invocation {
            executable: self.executable.clone(),
            arguments,
            directory: request.root.clone(),
        })
    }
    fn decode(&self, request: &RunRequest, output: ProcessOutput) -> Result<RunReport, Error> {
        if !matches!(output.status, 0 | 1) || !output.stderr.is_empty() {
            return Err(Error::Invalid(format!(
                "Ruff failed ({}): {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        let raw: Vec<Finding> = serde_json::from_slice(&output.stdout)?;
        if (output.status == 0) != raw.is_empty() {
            return Err(Error::Invalid(
                "Ruff exit status contradicts findings".into(),
            ));
        }
        let mut diagnostics = Vec::new();
        for finding in raw {
            if finding.cell.is_some() {
                return Err(Error::Invalid(
                    "notebook cell locations need a mapped source model".into(),
                ));
            }
            if !self.defaults.contains(&finding.code) && finding.code != "invalid-syntax" {
                return Err(Error::Invalid(format!("unexpected rule {}", finding.code)));
            }
            let filename = PathBuf::from(finding.filename);
            let relative = filename
                .strip_prefix(&request.root)
                .map_err(|_| Error::Invalid("Ruff returned a path outside the corpus".into()))?;
            let file = relative
                .to_str()
                .ok_or_else(|| Error::Invalid("non-UTF8 Ruff filename".into()))?
                .to_owned();
            if !request.files.contains(&file) {
                return Err(Error::Invalid("Ruff returned an unrequested file".into()));
            }
            let source = std::fs::read_to_string(filename)?;
            diagnostics.push(Diagnostic {
                rule: finding.code,
                file,
                start: offset(&source, &finding.location)?,
                end: offset(&source, &finding.end_location)?,
            });
        }
        Ok(RunReport {
            diagnostics,
            files: request.files.clone(),
        })
    }
}

impl Ruff {
    /// Ask the pinned tool for its default file selection before either timer starts.
    pub(crate) fn files(&self, root: &std::path::Path) -> Result<Vec<String>, Error> {
        let output = std::process::Command::new(&self.executable)
            .args(["check", "--isolated", "--show-files", "."])
            .current_dir(root)
            .output()?;
        if !output.status.success() || !output.stderr.is_empty() {
            return Err(Error::Invalid(format!(
                "Ruff file discovery failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        let text = std::str::from_utf8(&output.stdout)
            .map_err(|error| Error::Invalid(error.to_string()))?;
        let mut files = Vec::new();
        for line in text.lines() {
            let path = std::path::Path::new(line)
                .strip_prefix(root)
                .map_err(|_| Error::Invalid("discovered file outside corpus".into()))?;
            files.push(
                path.to_str()
                    .ok_or_else(|| Error::Invalid("non-UTF8 corpus path".into()))?
                    .to_owned(),
            );
        }
        files.sort();
        if files.is_empty() {
            return Err(Error::Invalid("empty discovered corpus".into()));
        }
        Ok(files)
    }
}
