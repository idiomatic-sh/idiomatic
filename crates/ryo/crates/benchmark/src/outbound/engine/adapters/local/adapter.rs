//! Query loading, compilation and SDK startup occur inside the measured child.
use super::constants;
use crate::outbound::engine::types::FrontendDiagnosticProjection;
use crate::outbound::engine::types::loaded_engine_session::value::EngineExecution;
use crate::outbound::engine::types::{
    EngineConfiguration, EngineRun, loaded_engine_session::value::LoadedEngineSession,
};
use ryo::linter::{
    self, Cache, Cell, Corpus, Languages, LintRequest, MemoryBudget, Rule, RuleId, Slot,
    WorkerPool, Workers,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::{BenchmarkFinding, PairCount};
use ryo_benchmark_contracts::domain::reports::types::RunReport;
use ryo_benchmark_contracts::domain::requests::types::WorkerRequest;
use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use tracing_subscriber::prelude::*;

use crate::outbound::engine::interface::EnginePort;

/// Runs authored queries through the SDK with the injected language registry.
#[derive(Debug)]
pub(crate) struct EngineLocalAdapter {
    /// Frontends selected by the outer language composition.
    languages: Languages,
    /// Optional trace destination for a separately labelled profiling invocation.
    trace: Option<std::path::PathBuf>,
    /// Optional flushed JSONL event destination for a separately labelled diagnostic invocation.
    diagnostic_events: Option<std::path::PathBuf>,
    /// Optional explicit query mapping provider diagnostics to benchmark findings.
    frontend_diagnostic: Option<FrontendDiagnosticProjection>,
}
impl EngineLocalAdapter {
    /// Bind language support and optional profiling output once at composition.
    pub(crate) fn new(
        languages: Languages,
        trace: Option<std::path::PathBuf>,
        frontend_diagnostic: Option<FrontendDiagnosticProjection>,
    ) -> Self {
        Self {
            languages,
            trace,
            diagnostic_events: None,
            frontend_diagnostic,
        }
    }
    /// Bind an explicit live JSONL diagnostic sink without changing the timing worker.
    pub(crate) fn with_diagnostic_events(
        languages: Languages,
        path: std::path::PathBuf,
        frontend_diagnostic: Option<FrontendDiagnosticProjection>,
    ) -> Self {
        Self {
            languages,
            trace: None,
            diagnostic_events: Some(path),
            frontend_diagnostic,
        }
    }
}
impl EnginePort for EngineLocalAdapter {
    fn load<'a>(&'a self, request: &'a WorkerRequest) -> Result<LoadedEngineSession<'a>, Error> {
        let trace = if let Some(path) = &self.trace {
            let (layer, guard) = tracing_chrome::ChromeLayerBuilder::new().file(path).build();
            tracing::subscriber::set_global_default(
                tracing_subscriber::registry()
                    .with(tracing_subscriber::filter::LevelFilter::INFO)
                    .with(layer),
            )
            .map_err(|error| Error::Invalid(error.to_string()))?;
            Some(guard)
        } else if let Some(path) = &self.diagnostic_events {
            tracing::subscriber::set_global_default(super::writer::subscriber(
                path,
                request.performance.diagnostics,
            )?)
            .map_err(|error| Error::Invalid(error.to_string()))?;
            None
        } else {
            None
        };
        let rules = super::schemas::load_rules(
            &request.rules,
            &request.query_modules,
            &request.rule_applicability,
            self.frontend_diagnostic.clone(),
            match &request.rule_observations {
                ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Declared(declared) => Some(declared),
                ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared => None,
            },
        )?;
        Ok(LoadedEngineSession::new(
            request,
            rules,
            self.frontend_diagnostic
                .as_ref()
                .map(|projection| projection.rule.clone()),
            Box::new(LocalEngineSession {
                adapter: self,
                request,
                _trace: trace,
            }),
        ))
    }
}

/// One loaded SDK invocation, never stored back into the reusable adapter.
struct LocalEngineSession<'a> {
    adapter: &'a EngineLocalAdapter,
    request: &'a WorkerRequest,
    // Last field: abandonment drops loaded rules before flushing the trace.
    _trace: Option<tracing_chrome::FlushGuard>,
}
impl EngineExecution for LocalEngineSession<'_> {
    fn configuration_files(&self) -> Result<Vec<linter::RelativePath>, Error> {
        configuration_files(self.request, &self.adapter.languages)
    }
    fn execute(
        self: Box<Self>,
        rules: BTreeMap<RuleId, Rule>,
        prepared: EngineConfiguration,
    ) -> Result<EngineRun, Error> {
        // Keep the named guard until this function returns, including SDK/report errors.
        let LocalEngineSession {
            adapter,
            request,
            _trace,
        } = *self;
        let crate::outbound::engine::types::EngineConfiguration {
            selected_missing,
            selected_rules,
            selected_pairs,
            hosted,
        } = prepared;
        let selected_pairs = selected_pairs.0;
        let (sources, accepted) = super::schemas::source_inputs(&request, &selected_missing)?;
        let pool = WorkerPool::from(Workers(NonZeroUsize::MIN));
        let cache = match &request.state {
            ryo_benchmark_contracts::domain::requests::types::WorkerState::Fresh => Cache::None,
            ryo_benchmark_contracts::domain::requests::types::WorkerState::Retained(path) => {
                Cache::At(path.clone())
            }
        };
        let (corpus, beside) = match request.project_context_mode {
            ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::Isolated => {
                (Corpus::Given(sources), vec![])
            }
            ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::ListedDisk => {
                (Corpus::Listed(request.files.clone()), sources)
            }
        };
        let retained_rules: std::collections::BTreeSet<RuleId> = rules.keys().cloned().collect();
        let engine = linter::Linter::new(
            adapter.languages.clone(),
            pool,
            MemoryBudget::Machine,
            cache,
        );
        let report = linter::lint_with(
            &engine,
            LintRequest {
                source_language: match &request.source_language {
                    ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Unspecified => ryo::linter::SourceLanguage::Recognized,
                    ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Selected(name) => ryo::linter::SourceLanguage::Named(
                        ryo::languages::LanguageName(name.0.clone().into()),
                    ),
                },
                root: request.root.clone(),
                project_search_scope: match &request.project_search_roots {
                    ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared => ryo::linter::ProjectSearchScope::Undeclared,
                    ryo_benchmark_contracts::domain::requests::types::SearchRoots::Declared(roots) => {
                        ryo::linter::ProjectSearchScope::Declared(roots.clone())
                    }
                },
                given_project_inventory: request.given_project_inventory.clone(),
                rules,
                files: vec![],
                hosted,
                corpus,
                beside,
            },
        )
        .map_err(|error| Error::Invalid(error.to_string()))?;
        let unexpected_unread = report.unread.iter().any(|unread| {
            !matches!(&unread.reason, linter::Unreadable::Failed(_))
                || !selected_missing.contains(&unread.path)
        });
        if !report.unknowns.is_empty() || unexpected_unread {
            return Err(Error::Invalid(format!(
                "incomplete candidate: selected_rule_file_pairs={selected_pairs}; unknown_count={}; unread_count={}; unknowns={:?}; unread={:?}",
                report.unknowns.len(),
                report.unread.len(),
                report.unknowns,
                report.unread
            )));
        }
        let input_files: std::collections::BTreeSet<_> = accepted.iter().cloned().collect();
        super::schemas::validate_skips(
            &report.skipped,
            &retained_rules,
            &request.rule_applicability,
            &input_files,
            adapter.frontend_diagnostic.is_some(),
        )?;
        // A producer kept only for a selected rule answers no file, so its skips are reported no more than its findings are.
        let skipped: Vec<linter::Skipped> = report
            .skipped
            .into_iter()
            .filter(|skip| selected_rules.contains(&skip.rule))
            .collect();
        let skips_count = skipped.len();
        let diagnostics = diagnostic_findings(report.findings)?;
        let dependencies = report.dependencies;
        Ok(EngineRun { dependencies, report: RunReport {
            execution_metadata:
                ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata::Complete,
            project_context: ryo_benchmark_contracts::domain::reports::types::ProjectContext::NotRequested,
            diagnostics,
            skipped,
            coverage: ryo_benchmark_contracts::domain::reports::types::CoverageAccounting::Accounted(
                ryo_benchmark_contracts::domain::reports::types::RuleCoverage {
                    selected: PairCount(selected_pairs),
                    evaluated: PairCount(selected_pairs.checked_sub(skips_count).ok_or_else(|| {
                        Error::Invalid(constants::SKIPPED_RULE_PAIRS_EXCEED_THE_CONFIGURED.into())
                    })?),
                    skipped: PairCount(skips_count),
                    unknown_or_unread: PairCount(0),
                },
            ),
            files: accepted.into_iter().collect(),
        } })
    }
}

/// Decode the explicit file/span projection, refusing malformed candidate rows.
fn diagnostic_findings(findings: Vec<linter::Finding>) -> Result<Vec<BenchmarkFinding>, Error> {
    let mut diagnostics = Vec::new();
    for finding in findings {
        let Slot::Is(Cell::File(file)) = finding.subject else {
            return Err(Error::Invalid(constants::INVALID_SUBJECT.into()));
        };
        let spans: Vec<_> = finding
            .columns
            .iter()
            .filter_map(|column| {
                if column.name.0.as_ref() == constants::SPAN_COLUMN
                    && let Slot::Is(Cell::Span(span)) = &column.value
                {
                    return Some(*span);
                }
                None
            })
            .collect();
        let [span] = spans.as_slice() else {
            return Err(Error::Invalid(constants::INVALID_SPAN.into()));
        };
        diagnostics.push(BenchmarkFinding {
            rule: finding.rule,
            file,
            start: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(
                usize::try_from(span.start.0).map_err(|error| Error::Invalid(error.to_string()))?,
            ),
            end: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(
                usize::try_from(span.end.0).map_err(|error| Error::Invalid(error.to_string()))?,
            ),
        });
    }
    Ok(diagnostics)
}

/// Syntax diagnostics apply to source inputs; provider-declared project inputs have their own reader.
fn configuration_files(
    request: &WorkerRequest,
    languages: &Languages,
) -> Result<Vec<linter::RelativePath>, Error> {
    let selected_language = match &request.source_language {
        ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Selected(name) => Some(name.0.as_str()),
        ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Unspecified => None,
    };
    let matching: Vec<_> = match selected_language {
        Some(selected) => languages
            .0
            .iter()
            .filter(|language| language.name().0.as_ref() == selected)
            .collect(),
        None if languages.0.len() == 1 => languages.0.iter().collect(),
        // Automatic multi-provider parsing chooses the actual accepting source provider. Without an
        // explicit unique owner, do not infer a configuration exclusion from one provider alone.
        None => return Ok(Vec::new()),
    };
    let [language] = matching.as_slice() else {
        return Err(Error::Invalid(
            crate::outbound::engine::adapters::local::constants::EXPLICIT_PRIMARY_ROLE_PROVIDER_IS_MISSING.into(),
        ));
    };
    Ok(request
        .files
        .iter()
        .filter(|file| {
            language.primary_role(ryo::languages::SourcePath(&file.0))
                == ryo::languages::PrimaryFileRole::Configuration
        })
        .cloned()
        .collect())
}
