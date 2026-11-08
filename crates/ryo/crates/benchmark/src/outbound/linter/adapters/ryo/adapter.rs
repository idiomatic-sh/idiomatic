//! Serialized request files avoid shell interpretation and OS argument-size limits.
use crate::outbound::linter::adapters::ryo::constants;
use crate::outbound::linter::interface::LinterPort;
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
use ryo_benchmark_contracts::domain::measurements::types::ProcessOutput;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::RunReport;
use ryo_benchmark_contracts::domain::requests::types::RunRequest;
use ryo_benchmark_contracts::domain::requests::types::{
    ApplicabilityPolicy, CohortSelection, ContextSeal, InputDeclaration, ParserSelection,
    RuleLabel, WorkerRequest,
};
use super::types::{ReportBytes, RequestJson};
use crate::outbound::linter::types::CanonicalText;
use ryo_benchmark_contracts::domain::context::types::PreparedRuleContext;
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::reports::types::ProjectContext;
use ryo_benchmark_contracts::domain::workloads::types::{PreparedCase, Tool};
use std::path::PathBuf;
#[derive(Debug)]
/// Constructs explicit candidate invocations and validates their reports.
pub(crate) struct LinterRyoAdapter {
    /// Frozen default cohort; supporting query definitions are not implicitly enabled.
    pub defaults: Vec<ryo::linter::RuleId>,
    /// Explicit profile declarations; supporting definitions alone never bind a consumer.
    pub rule_observation_selection:
        ryo_benchmark_contracts::domain::context::types::ObservationSelectionPolicy,
    pub rule_observations:
        ryo_benchmark_contracts::domain::context::types::rule_observation_declarations::value::RuleObservationDeclarations,
    /// Explicit isolation contract selected by the frozen profile.
    pub project_context_mode: ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode,
    /// Pinned candidate executable.
    pub executable: PathBuf,
    /// Frozen directory of selected rule queries.
    pub rules: PathBuf,
    /// Explicit logical query-module registry under the frozen profile root.
    pub query_modules: std::collections::BTreeMap<ryo_benchmark_contracts::domain::requests::types::QueryModuleName, PathBuf>,
    /// Exact selected-rule syntax applicability policy from the frozen profile.
    pub rule_applicability: std::collections::BTreeMap<ryo_benchmark_contracts::domain::requests::types::RuleLabel, ryo_benchmark_contracts::domain::requests::types::ApplicabilityPolicy>,
    /// Parser configuration forwarded to the isolated SDK worker.
    pub parser_configuration: ryo_benchmark_contracts::domain::requests::types::ParserSelection,
    /// Explicit source provider selected by the request edge, if any.
    pub source_language: ryo_benchmark_contracts::domain::requests::types::ProviderSelection,
}
impl LinterPort for LinterRyoAdapter {
    fn rule_configuration(
        &self,
    ) -> ryo_benchmark_contracts::domain::workloads::types::RuleConfigurationSupport {
        ryo_benchmark_contracts::domain::workloads::types::RuleConfigurationSupport::Explicit
    }
    fn prepare(&self, request: &RunRequest) -> Result<Invocation, Error> {
        let selected_rules = match &request.selected_rules {
            CohortSelection::Frozen => self.defaults.clone(),
            CohortSelection::Explicit(rules) => rules.clone(),
        };
        let profile_observations = self
            .rule_observations
            .project_profile(&selected_rules, self.rule_observation_selection);
        let worker = WorkerRequest {
            performance: request.performance,
            rule_context: request.rule_context.clone(),
            rule_observations: match &request.rule_observations {
                InputDeclaration::Declared(declarations) => {
                    InputDeclaration::Declared(declarations.clone())
                }
                InputDeclaration::Undeclared if profile_observations.is_empty() => {
                    InputDeclaration::Undeclared
                }
                InputDeclaration::Undeclared => InputDeclaration::Declared(profile_observations),
            },
            selected_rules: CohortSelection::Explicit(selected_rules),
            project_search_roots: request.project_search_roots.clone(),
            given_project_inventory: request.given_project_inventory.clone(),
            project_context_mode: self.project_context_mode,
            context_manifest: request.context_manifest.clone(),
            root: request.root.clone(),
            rules: self.rules.clone(),
            query_modules: self.query_modules.clone(),
            rule_applicability: self.rule_applicability.clone(),
            parser_configuration: self.parser_configuration.clone(),
            source_language: self.source_language.clone(),
            files: request.files.clone(),
            state: ryo_benchmark_contracts::domain::requests::types::WorkerState::Retained(
                request.state.join(super::constants::CACHE),
            ),
        };
        let request_json = RequestJson::try_from(worker)?;
        let request_path = request.state.join(super::constants::REQUEST_FILENAME);
        std::fs::create_dir_all(&request.state)?;
        std::fs::write(&request_path, request_json.0)?;
        let mut arguments = vec![
            super::constants::WORKER.into(),
            super::constants::REQUEST_FILE.into(),
            request_path.into_os_string(),
        ];
        if let ParserSelection::Named(configuration) = &self.parser_configuration {
            arguments.push(configuration.0.clone().into());
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
        if output.status.0 != 0_i32 || !output.stderr.0.is_empty() {
            return Err(Error::Invalid(format!(
                "candidate failed ({}): {}",
                output.status.0,
                String::from_utf8_lossy(&output.stderr.0)
            )));
        }
        let report = RunReport::try_from(ReportBytes(&output.stdout.0))?;
        for skip in &report.skipped {
            let label = RuleLabel(skip.rule.as_ref().to_owned());
            if self.rule_applicability.get(&label)
                != Some(&ApplicabilityPolicy(constants::ACCEPTED_SYNTAX.to_owned()))
            {
                return Err(Error::Invalid(
                    constants::CANDIDATE_SKIPPED_AN_UNDECLARED_OR_ALWAYS.into(),
                ));
            }
        }
        let consistent = match (
            self.project_context_mode,
            &request.context_manifest,
            &report.project_context,
        ) {
            (ProjectContextMode::Isolated, ContextSeal::Unsealed, ProjectContext::NotRequested)
            | (ProjectContextMode::ListedDisk, ContextSeal::Sealed(_), ProjectContext::Observed(_)) => true,
            (ProjectContextMode::Isolated, ContextSeal::Unsealed, ProjectContext::Observed(_))
            | (ProjectContextMode::Isolated, ContextSeal::Sealed(_), _)
            | (ProjectContextMode::ListedDisk, ContextSeal::Unsealed, _)
            | (ProjectContextMode::ListedDisk, ContextSeal::Sealed(_), ProjectContext::NotRequested) => false,
        };
        if !consistent {
            return Err(Error::Invalid(
                constants::CANDIDATE_CONTEXT_MODE_MANIFEST_AND_RECEIPT.into(),
            ));
        }
        Ok(report)
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
}
