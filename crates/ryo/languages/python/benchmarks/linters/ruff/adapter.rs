//! Ruff owns its invocation and requested-source IO; schemas normalize its process response.
use super::{constants, schemas, settings::LinterRuffSettings};
use ryo::linter::{RelativePath, RuleId, SourceText};
use ryo_benchmark::domain::measurements::types::{Invocation, ProcessOutput};
use ryo_benchmark::domain::{
    diagnostics::{errors::Error, types::RunReport},
    suite::{service::relative_path, types::RunRequest},
};
use ryo_benchmark::outbound::linter::interface::LinterPort;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub(crate) struct LinterRuffAdapter {
    executable: PathBuf,
    defaults: BTreeSet<RuleId>,
}
impl LinterPort for LinterRuffAdapter {
    fn prepare(&self, request: &RunRequest) -> Result<Invocation, Error> {
        if request.files.is_empty() {
            return Err(Error::Invalid(constants::EMPTY_INPUT.into()));
        }
        let mut arguments = vec![
            constants::CHECK.into(),
            constants::ISOLATED.into(),
            constants::OUTPUT_FORMAT.into(),
            constants::JSON.into(),
            constants::CACHE_DIR.into(),
            request.cache.join(constants::CACHE_NAME).into_os_string(),
            constants::END_OPTIONS.into(),
        ];
        for file in &request.files {
            relative_path(file)?;
            arguments.push(file.into());
        }
        Ok(Invocation {
            executable: self.executable.clone(),
            arguments,
            environment: [(constants::WORKERS_ENV.into(), constants::ONE_WORKER.into())]
                .into_iter()
                .collect(),
            directory: request.root.clone(),
        })
    }
    fn decode(&self, request: &RunRequest, output: ProcessOutput) -> Result<RunReport, Error> {
        schemas::check_status(&output)?;
        let mut sources = BTreeMap::new();
        for file in &request.files {
            relative_path(file)?;
            let key = RelativePath(file.clone().into_boxed_str());
            let source =
                SourceText(std::fs::read_to_string(request.root.join(file))?.into_boxed_str());
            if sources.insert(key, source).is_some() {
                return Err(Error::Invalid(constants::DUPLICATE_INPUT.into()));
            }
        }
        schemas::decode(request, &output, &self.defaults, &sources)
    }
}

impl LinterRuffAdapter {
    pub(crate) fn new(settings: LinterRuffSettings) -> Self {
        Self {
            executable: settings.executable,
            defaults: settings.defaults,
        }
    }

    /// Ask the pinned tool for its default file selection before either timer starts.
    pub(crate) fn files(&self, root: &Path) -> Result<Vec<RelativePath>, Error> {
        let output = std::process::Command::new(&self.executable)
            .args([
                constants::CHECK,
                constants::ISOLATED,
                constants::SHOW_FILES,
                constants::CURRENT_DIRECTORY,
            ])
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
            let path = Path::new(line)
                .strip_prefix(root)
                .map_err(|_| Error::Invalid(constants::DISCOVERED_OUTSIDE.into()))?;
            let file = path
                .to_str()
                .ok_or_else(|| Error::Invalid(constants::NON_UTF8_PATH.into()))?;
            relative_path(file)?;
            files.push(RelativePath(file.into()));
        }
        files.sort();
        if files.is_empty() {
            return Err(Error::Invalid(constants::EMPTY_DISCOVERY.into()));
        }
        Ok(files)
    }
}
