//! oxlint owns its invocation; schemas normalize its process response.
use super::{constants, schemas, settings::LinterOxlintSettings};
use ryo::linter::RuleId;
use ryo_benchmark::outbound::linter::interface::LinterPort;
use ryo_benchmark::outbound::linter::types::CanonicalText;
use ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath;
use ryo_benchmark_contracts::domain::context::types::PreparedRuleContext;
use ryo_benchmark_contracts::domain::measurements::types::{Invocation, ProcessOutput};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::RunReport;
use ryo_benchmark_contracts::domain::requests::types::{
    CohortSelection, RuleContextPreparation, RunRequest,
};
use ryo_benchmark_contracts::domain::workloads::types::version_line::value::VersionLine;
use ryo_benchmark_contracts::domain::workloads::types::{PreparedCase, Tool};
use std::collections::BTreeSet;

#[derive(Debug)]
/// Executes the pinned oxlint protocol against an explicit source set.
pub(crate) struct LinterOxlintAdapter {
    /// Executable and permitted rules chosen by composition.
    settings: LinterOxlintSettings,
}
impl LinterPort for LinterOxlintAdapter {
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
        std::fs::create_dir_all(&request.state)?;
        let configuration = request.state.join(constants::CONFIG_NAME);
        std::fs::write(&configuration, constants::EMPTY_CONFIG)?;
        let mut arguments = vec![
            constants::CONFIG.into(),
            configuration.into_os_string(),
            constants::ALLOW.into(),
            constants::ALL.into(),
        ];
        for rule in &self.selected(request) {
            arguments.push(constants::DENY.into());
            arguments.push(schemas::selector_of_rule(rule)?.into());
        }
        arguments.extend(
            [
                constants::FORMAT,
                constants::JSON,
                constants::THREADS,
                constants::ONE_WORKER,
                constants::NO_NESTED_CONFIG,
                constants::END_OPTIONS,
            ]
            .map(Into::into),
        );
        let mut seen = BTreeSet::new();
        for file in &request.files {
            let _relative = RelativeArtifactPath::try_from(std::path::Path::new(file.0.as_ref()))?;
            if !seen.insert(file) {
                return Err(Error::Invalid(constants::DUPLICATE_INPUT.into()));
            }
            arguments.push(file.0.as_ref().into());
        }
        Ok(Invocation {
            executable: self.settings.executable.clone(),
            arguments,
            environment: [(
                ryo_benchmark_contracts::domain::measurements::constants::THREAD_PIN.into(),
                constants::ONE_WORKER.into(),
            )]
            .into_iter()
            .collect(),
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
        VersionLine(format!("{}{}", constants::VERSION_PREFIX, tool.version).into())
    }
}

impl LinterOxlintAdapter {
    /// Bind the executable and selected rules before preparing invocations.
    pub(crate) fn new(settings: LinterOxlintSettings) -> Self {
        Self { settings }
    }
    /// The rules one request runs: the pinned defaults, or the ones it names.
    fn selected(&self, request: &RunRequest) -> BTreeSet<RuleId> {
        match &request.selected_rules {
            CohortSelection::Frozen => self.settings.defaults.clone(),
            CohortSelection::Explicit(rules) => rules.iter().cloned().collect(),
        }
    }
}
