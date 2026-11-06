//! Serialized request files avoid shell interpretation and OS argument-size limits.
use crate::outbound::linter::interface::LinterPort;
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
use ryo_benchmark_contracts::domain::measurements::types::ProcessOutput;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::RunReport;
use ryo_benchmark_contracts::domain::requests::types::RunRequest;
use ryo_benchmark_contracts::domain::requests::types::WorkerRequest;
use std::path::PathBuf;
#[derive(Debug)]
/// Constructs explicit candidate invocations and validates their reports.
pub(crate) struct LinterRyoAdapter {
    /// Frozen default cohort; supporting query definitions are not implicitly enabled.
    pub defaults: Vec<ryo::linter::RuleId>,
    /// Explicit profile declarations; supporting definitions alone never bind a consumer.
    pub rule_observation_selection:
        ryo_benchmark_contracts::domain::workloads::types::ObservationSelectionPolicy,
    pub rule_observations:
        ryo_benchmark_contracts::domain::context::types::rule_observation_declarations::value::RuleObservationDeclarations,
    /// Explicit isolation contract selected by the frozen profile.
    pub project_context_mode: ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode,
    /// Pinned candidate executable.
    pub executable: PathBuf,
    /// Frozen directory of selected rule queries.
    pub rules: PathBuf,
    /// Explicit logical query-module registry under the frozen profile root.
    pub query_modules: std::collections::BTreeMap<String, PathBuf>,
    /// Exact selected-rule syntax applicability policy from the frozen profile.
    pub rule_applicability: std::collections::BTreeMap<String, String>,
    /// Parser configuration forwarded to the isolated SDK worker.
    pub parser_configuration: Option<String>,
    /// Explicit source provider selected by the request edge, if any.
    pub source_language: Option<String>,
}
impl LinterPort for LinterRyoAdapter {
    fn rule_configuration(
        &self,
    ) -> ryo_benchmark_contracts::domain::workloads::types::RuleConfigurationSupport {
        ryo_benchmark_contracts::domain::workloads::types::RuleConfigurationSupport::Explicit
    }
    fn prepare(&self, request: &RunRequest) -> Result<Invocation, Error> {
        let selected_rules = request
            .selected_rules
            .clone()
            .unwrap_or_else(|| self.defaults.clone());
        let profile_observations = crate::domain::suite::observations::project_profile(
            &self.rule_observations,
            &selected_rules,
            self.rule_observation_selection,
        );
        let worker = WorkerRequest {
            performance: request.performance,
            rule_context: request.rule_context.clone(),
            rule_observations: request
                .rule_observations
                .clone()
                .or_else(|| (!profile_observations.is_empty()).then_some(profile_observations)),
            selected_rules: Some(selected_rules),
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
        let request_json = super::schemas::encode_request(worker)?;
        let request_path = request.state.join(super::constants::REQUEST_FILENAME);
        std::fs::create_dir_all(&request.state)?;
        std::fs::write(&request_path, request_json)?;
        let mut arguments = vec![
            super::constants::WORKER.into(),
            super::constants::REQUEST_FILE.into(),
            request_path.into_os_string(),
        ];
        if let Some(configuration) = &self.parser_configuration {
            arguments.push(configuration.clone().into());
        }
        Ok(Invocation {
            executable: self.executable.clone(),
            arguments,
            environment: std::collections::BTreeMap::default(),
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
        let report = super::schemas::decode_report(&output.stdout.0)?;
        for skip in &report.skipped {
            if self
                .rule_applicability
                .get(skip.rule.as_ref())
                .map(String::as_str)
                != Some(crate::outbound::linter::adapters::ryo::constants::ACCEPTED_SYNTAX)
            {
                return Err(Error::Invalid(
                    crate::outbound::linter::adapters::ryo::constants::CANDIDATE_SKIPPED_AN_UNDECLARED_OR_ALWAYS.into(),
                ));
            }
        }
        match (
            self.project_context_mode,
            &request.context_manifest,
            &report.project_context,
        ) {
            (
                ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::Isolated,
                None,
                None,
            ) => {}
            (
                ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::ListedDisk,
                Some(manifest),
                Some(receipt),
            ) => crate::outbound::context::adapters::local::adapter::verify_receipt(
                manifest,
                &request.root,
                receipt,
            )?,
            _ => {
                return Err(Error::Invalid(
                    crate::outbound::linter::adapters::ryo::constants::CANDIDATE_CONTEXT_MODE_MANIFEST_AND_RECEIPT.into(),
                ));
            }
        }
        Ok(report)
    }
}
