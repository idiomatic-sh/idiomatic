//! Ruff owns its invocation and requested-source IO; schemas normalize its process response.
use super::{constants, schemas, settings::LinterRuffSettings};
use ryo::linter::{RuleId, SourceText};
use ryo_benchmark::outbound::linter::interface::LinterPort;
use ryo_benchmark::outbound::linter::types::CanonicalText;
use ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath;
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
use ryo_benchmark_contracts::domain::measurements::types::ProcessOutput;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::RunReport;
use ryo_benchmark_contracts::domain::requests::types::RunRequest;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug)]
/// Executes the pinned Ruff protocol against an explicit source set.
pub(crate) struct LinterRuffAdapter {
    /// Executable, permitted rules and target chosen by composition.
    settings: LinterRuffSettings,
}
impl LinterPort for LinterRuffAdapter {
    fn rule_configuration(
        &self,
    ) -> ryo_benchmark_contracts::domain::workloads::types::RuleConfigurationSupport {
        ryo_benchmark_contracts::domain::workloads::types::RuleConfigurationSupport::Explicit
    }
    fn prepare(&self, request: &RunRequest) -> Result<Invocation, Error> {
        if request.files.is_empty() {
            return Err(Error::Invalid(constants::EMPTY_INPUT.into()));
        }
        // The pinned isolated Ruff profile uses its ordinary repository and src roots.
        // A different host declaration would compare different project categorization.
        if matches!(&request.project_search_roots, ryo_benchmark_contracts::domain::requests::types::SearchRoots::Declared(roots) if !roots
                .iter()
                .map(|root| root.0.as_ref())
                .eq(constants::ISOLATED_PROJECT_ROOTS))
        {
            return Err(Error::Invalid(constants::UNSUPPORTED_PROJECT_ROOTS.into()));
        }
        let selected = match &request.selected_rules {
            ryo_benchmark_contracts::domain::requests::types::CohortSelection::Frozen => {
                self.settings.defaults.clone()
            }
            ryo_benchmark_contracts::domain::requests::types::CohortSelection::Explicit(rules) => {
                rules.iter().cloned().collect()
            }
        };
        let mut arguments = vec![
            constants::CHECK.into(),
            constants::ISOLATED.into(),
            constants::OUTPUT_FORMAT.into(),
            constants::JSON.into(),
            constants::CACHE_DIR.into(),
            request.state.join(constants::CACHE_NAME).into_os_string(),
            constants::SELECT.into(),
            selected
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(constants::RULE_SEPARATOR)
                .into(),
            constants::TARGET_VERSION.into(),
            self.settings.target_version.clone(),
        ];
        if let ryo_benchmark_contracts::domain::requests::types::RuleContextPreparation::Prepared(
            context,
        ) = &request.rule_context
        {
            let root = std::path::absolute(&request.root)?;
            arguments.extend(schemas::configuration_arguments(
                &context.configuration,
                &root,
            )?);
        }
        arguments.push(constants::END_OPTIONS.into());
        for file in &request.files {
            RelativeArtifactPath::try_from(std::path::Path::new(file.0.as_ref()))?;
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
        schemas::check_status(&output)?;
        let mut sources = BTreeMap::new();
        for file in &request.files {
            RelativeArtifactPath::try_from(std::path::Path::new(file.0.as_ref()))?;
            let key = file.clone();
            if matches!(&request.context_manifest, ryo_benchmark_contracts::domain::requests::types::ContextSeal::Sealed(manifest) if
                ryo_benchmark::domain::suite::service::declares_missing(manifest, file) == ryo_benchmark::domain::suite::types::ContextAbsence::Sealed)
            {
                // This is a declared, reverified absence, not empty source text.
                continue;
            }
            let source = SourceText(
                std::fs::read_to_string(request.root.join(file.0.as_ref()))?.into_boxed_str(),
            );
            if sources.insert(key, source).is_some() {
                return Err(Error::Invalid(constants::DUPLICATE_INPUT.into()));
            }
        }
        let selected = match &request.selected_rules {
            ryo_benchmark_contracts::domain::requests::types::CohortSelection::Frozen => {
                self.settings.defaults.clone()
            }
            ryo_benchmark_contracts::domain::requests::types::CohortSelection::Explicit(rules) => {
                rules.iter().cloned().collect()
            }
        };
        schemas::decode(request, &output, &selected, &sources)
    }
    fn settings_text(
        &self,
        normalized: &Invocation,
        case: &ryo_benchmark_contracts::domain::workloads::types::PreparedCase,
        profile: &ryo_benchmark_contracts::domain::workloads::types::Tool,
    ) -> Result<CanonicalText, Error> {
        CanonicalText::try_from((normalized, case, profile))
    }
    fn rule_context_text(
        &self,
        prepared: &ryo_benchmark_contracts::domain::context::types::PreparedRuleContext,
    ) -> CanonicalText {
        CanonicalText::from(prepared)
    }
    fn version_line(
        &self,
        tool: &ryo_benchmark_contracts::domain::workloads::types::Tool,
    ) -> ryo_benchmark_contracts::domain::workloads::types::version_line::value::VersionLine {
        ryo_benchmark_contracts::domain::workloads::types::version_line::value::VersionLine(
            format!("{} {}", tool.tool, tool.version).into(),
        )
    }
}

impl LinterRuffAdapter {
    /// Bind the executable and selected rules before preparing invocations.
    pub(crate) fn new(settings: LinterRuffSettings) -> Self {
        Self { settings }
    }
}
