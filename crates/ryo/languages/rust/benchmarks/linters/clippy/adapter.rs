//! clippy owns its invocation; schemas normalize its process response.
use super::{constants, schemas, settings::LinterClippySettings};
use ryo::linter::{RelativePath, RuleId};
use ryo_benchmark::outbound::discovery::interface::DiscoveryPort;
use ryo_benchmark::outbound::linter::interface::LinterPort;
use ryo_benchmark::outbound::linter::types::CanonicalText;
use ryo_benchmark_contracts::domain::context::types::PreparedRuleContext;
use ryo_benchmark_contracts::domain::measurements::types::{Invocation, ProcessOutput};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::RunReport;
use ryo_benchmark_contracts::domain::requests::types::{
    CohortSelection, RuleContextPreparation, RunRequest,
};
use ryo_benchmark_contracts::domain::workloads::types::version_line::value::VersionLine;
use ryo_benchmark_contracts::domain::workloads::types::{PreparedCase, Tool};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::path::Path;

#[derive(Debug)]
/// Executes the pinned clippy protocol: the check of a root's workspace, every target, the selected lints only.
pub(crate) struct LinterClippyAdapter {
    /// Executable, permitted rules and build directory chosen by composition.
    settings: LinterClippySettings,
}
impl LinterPort for LinterClippyAdapter {
    fn rule_configuration(
        &self,
    ) -> ryo_benchmark_contracts::domain::workloads::types::RuleConfigurationSupport {
        ryo_benchmark_contracts::domain::workloads::types::RuleConfigurationSupport::Unsupported
    }
    fn prepare(&self, request: &RunRequest) -> Result<Invocation, Error> {
        if request.files.is_empty() {
            return Err(Error::Invalid(constants::EMPTY_INPUT.into()));
        }
        if matches!(request.rule_context, RuleContextPreparation::Prepared(_)) {
            return Err(Error::Invalid(constants::UNSUPPORTED_RULE_CONTEXT.into()));
        }
        case_manifest(&request.root)?;
        touch_sources(request)?;
        let mut arguments: Vec<OsString> = constants::MEASURED_RUN.map(Into::into).into();
        for rule in &self.selected(request) {
            arguments.push(constants::WARN.into());
            arguments.push(schemas::selector_of_rule(rule)?.into());
        }
        Ok(Invocation {
            executable: self
                .settings
                .executable
                .with_file_name(constants::CLIPPY_COMMAND),
            arguments,
            environment: self.environment(),
            directory: request.root.clone(),
        })
    }
    fn decode(&self, request: &RunRequest, output: ProcessOutput) -> Result<RunReport, Error> {
        schemas::decode(request, &output, &self.selected(request))
    }
    fn settings_text(
        &self,
        normalized: &Invocation,
        case: &PreparedCase,
        profile: &Tool,
    ) -> Result<CanonicalText, Error> {
        CanonicalText::try_from((normalized, case, profile))
    }
    fn rule_context_text(&self, prepared: &PreparedRuleContext) -> CanonicalText {
        CanonicalText::from(prepared)
    }
    fn version_line(&self, tool: &Tool) -> VersionLine {
        VersionLine(
            format!(
                "{}{}{}",
                constants::VERSION_PREFIX,
                tool.version,
                constants::VERSION_STAMP
            )
            .into(),
        )
    }
}
impl DiscoveryPort for LinterClippyAdapter {
    fn files(&self, root: &Path) -> Result<Vec<RelativePath>, Error> {
        let started = std::time::SystemTime::now();
        let status = std::process::Command::new(
            self.settings
                .executable
                .with_file_name(constants::CLIPPY_COMMAND),
        )
        .args(constants::DISCOVERY_RUN)
        .envs(self.toolchain())
        .current_dir(root)
        .status()?;
        if !status.success() {
            return Err(Error::Invalid(constants::DISCOVERY_FAILED.into()));
        }
        let mut files = BTreeSet::new();
        for package in std::fs::read_dir(self.settings.targets.join(constants::DEP_INFO_DIRECTORY))?
        {
            for unit in std::fs::read_dir(package?.path())? {
                for entry in std::fs::read_dir(unit?.path().join(constants::UNIT_OUTPUT_DIRECTORY))?
                {
                    let entry = entry?;
                    let path = entry.path();
                    // A command's targets are shared by every root it runs in: the dependency files an earlier root's
                    // discovery wrote name that root's sources relative to it, so only the ones this run wrote are read.
                    if path
                        .extension()
                        .is_some_and(|extension| extension == constants::DEP_INFO_EXTENSION)
                        && entry.metadata()?.modified()? >= started
                    {
                        files.extend(schemas::dep_info_sources(
                            &std::fs::read_to_string(&path)?,
                            root,
                        )?);
                    }
                }
            }
        }
        if files.is_empty() {
            return Err(Error::Invalid(constants::NOTHING_DISCOVERED.into()));
        }
        Ok(files.into_iter().collect())
    }
}

impl LinterClippyAdapter {
    /// Bind the executable, selected rules and build directory before preparing invocations.
    pub(crate) fn new(settings: LinterClippySettings) -> Self {
        Self { settings }
    }
    /// The rules one request runs: the pinned defaults, or the ones it names.
    fn selected(&self, request: &RunRequest) -> BTreeSet<RuleId> {
        match &request.selected_rules {
            CohortSelection::Frozen => self.settings.defaults.clone(),
            CohortSelection::Explicit(rules) => rules.iter().cloned().collect(),
        }
    }
    /// The pinned toolchain's package manager and rustc, the shared target directory, no project clippy.toml: the environment
    /// of every run, discovery's included.
    fn toolchain(&self) -> BTreeMap<OsString, OsString> {
        [
            (
                constants::MANAGER_VARIABLE.into(),
                self.settings
                    .executable
                    .with_file_name(constants::MANAGER)
                    .into_os_string(),
            ),
            (
                constants::RUSTC_VARIABLE.into(),
                self.settings
                    .executable
                    .with_file_name(constants::RUSTC)
                    .into_os_string(),
            ),
            (
                constants::TARGET_VARIABLE.into(),
                self.settings.targets.clone().into_os_string(),
            ),
            (
                constants::CONF_VARIABLE.into(),
                self.settings.targets.clone().into_os_string(),
            ),
        ]
        .into_iter()
        .collect()
    }
    /// The toolchain's environment with one job and one worker: the runs `prepare` builds, measured or compared. Discovery,
    /// which checks every dependency once, runs at the default job count.
    fn environment(&self) -> BTreeMap<OsString, OsString> {
        let mut environment = self.toolchain();
        environment.extend([
            (constants::JOBS_VARIABLE.into(), constants::ONE_JOB.into()),
            (
                ryo_benchmark_contracts::domain::measurements::constants::THREAD_PIN.into(),
                constants::ONE_JOB.into(),
            ),
        ]);
        environment
    }
}

/// Write the case crate's manifest into a root that has none, so a case is a crate the check builds; a project's own is kept.
fn case_manifest(root: &Path) -> Result<(), Error> {
    match std::fs::File::options()
        .write(true)
        .create_new(true)
        .open(root.join(constants::MANIFEST_NAME))
    {
        Ok(mut file) => Ok(std::io::Write::write_all(
            &mut file,
            schemas::case_manifest()?.as_bytes(),
        )?),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error.into()),
    }
}

/// Mark every requested source modified now, so the check runs again on each crate that holds one and replays no cached message.
/// A requested file that is not there has nothing to mark: no crate the check builds reads it.
fn touch_sources(request: &RunRequest) -> Result<(), Error> {
    let now = std::time::SystemTime::now();
    for file in &request.files {
        match std::fs::File::options()
            .write(true)
            .open(request.root.join(file.0.as_ref()))
        {
            Ok(source) => source.set_modified(now)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}
