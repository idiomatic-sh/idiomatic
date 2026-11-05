//! Deterministic SDK substitute with a real request-scoped session contract.
use crate::outbound::engine::interface::EnginePort;
use crate::outbound::engine::types::loaded_engine_session::value::EngineExecution;
use crate::outbound::engine::types::{
    EngineConfiguration, loaded_engine_session::value::LoadedEngineSession,
};
use ryo::linter::{RelativePath, Rule, RuleId};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::RunReport;
use ryo_benchmark_contracts::domain::requests::types::WorkerRequest;
use std::collections::BTreeMap;

/// Deterministic engine substitute for orchestration and contract tests.
#[derive(Clone, Copy, Debug, Default)]
pub struct EngineFake {
    /// Whether query loading returns the shared missing-query refusal.
    pub refuses: bool,
}
impl EnginePort for EngineFake {
    fn load<'a>(
        &'a self,
        request: &'a WorkerRequest,
    ) -> Result<
        crate::outbound::engine::types::loaded_engine_session::value::LoadedEngineSession<'a>,
        Error,
    > {
        if self.refuses || request.selected_rules.as_ref().is_none_or(Vec::is_empty) {
            return Err(Error::Invalid("no candidate queries".into()));
        }
        Ok(LoadedEngineSession::new(
            request,
            loaded_rules(request)?,
            None,
            Box::new(FakeSession { request }),
        ))
    }
}
struct FakeSession<'a> {
    request: &'a WorkerRequest,
}
impl EngineExecution for FakeSession<'_> {
    fn configuration_files(&self) -> Result<Vec<RelativePath>, Error> {
        Ok(Vec::new())
    }
    fn execute(
        self: Box<Self>,
        _rules: BTreeMap<RuleId, Rule>,
        prepared: EngineConfiguration,
    ) -> Result<RunReport, Error> {
        Ok(RunReport {
            execution_metadata:
                ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata::Complete,
            project_context: None,
            files: self.request.files.clone(),
            skipped: Vec::new(),
            coverage: Some(
                ryo_benchmark_contracts::domain::reports::types::RuleCoverage {
                    selected: prepared.selected_pairs,
                    evaluated: prepared.selected_pairs,
                    skipped: 0,
                    unknown_or_unread: 0,
                },
            ),
            diagnostics: prepared.selected_missing.iter().flat_map(|file| prepared.selected_rules.iter().map(move |rule| ryo_benchmark_contracts::domain::reports::types::BenchmarkFinding { rule: rule.clone(), file: file.clone(), start: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(0), end: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(0) })).collect(),
        })
    }
}
/// Loaded metadata supplied by this fake; application rule policy still runs normally.
pub(crate) fn loaded_rules(request: &WorkerRequest) -> Result<BTreeMap<RuleId, Rule>, Error> {
    request.selected_rules.as_deref().unwrap_or(&[]).iter().map(|id| {
        let query = ryo::query::RyoQlQuery::try_from("let missing = select path as file, span from \"filesystem.read-failure\"; emit missing subject file key(file);")
            .map_err(|error| Error::Invalid(error.to_string()))?;
        Ok((id.clone(), Rule { query, observations: request.rule_observations.as_ref().and_then(|values| values.get(id)).cloned().unwrap_or_default(),
            applicability: ryo::linter::RuleApplicability::Always, parameters: BTreeMap::new(), include: vec![], exclude: vec![], }))
    }).collect()
}
#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
