//! Query loading, compilation and SDK startup occur inside the measured child.
use super::constants;
use crate::domain::diagnostics::{
    errors::DiagnosticsError as Error,
    types::{BenchmarkFinding, RunReport},
};
use crate::domain::suite::service::relative_path;
use crate::domain::suite::types::WorkerRequest;
use ryo::linter::{
    self, Cache, CaseSource, Cell, Corpus, Languages, LintRequest, MemoryBudget, Rule, RuleId,
    Slot, SourceText, WorkerPool, Workers,
};
use ryo::query::RyoQlQuery;
use std::collections::BTreeMap;
use std::num::NonZeroUsize;

use crate::outbound::engine::interface::EnginePort;

#[derive(Debug)]
pub(crate) struct EngineLocalAdapter {
    languages: Languages,
    trace: Option<std::path::PathBuf>,
}
impl EngineLocalAdapter {
    pub(crate) fn new(languages: Languages, trace: Option<std::path::PathBuf>) -> Self {
        Self { languages, trace }
    }
}
impl EnginePort for EngineLocalAdapter {
    fn run(&self, request: WorkerRequest) -> Result<RunReport, Error> {
        use tracing_subscriber::prelude::*;
        let _trace = if let Some(path) = &self.trace {
            let (layer, guard) = tracing_chrome::ChromeLayerBuilder::new().file(path).build();
            tracing::subscriber::set_global_default(
                tracing_subscriber::registry()
                    .with(tracing_subscriber::filter::LevelFilter::INFO)
                    .with(layer),
            )
            .map_err(|error| Error::Invalid(error.to_string()))?;
            Some(guard)
        } else {
            None
        };
        let mut rules = BTreeMap::new();
        for entry in std::fs::read_dir(&request.rules)? {
            let entry = entry?;
            let id = entry
                .file_name()
                .into_string()
                .map_err(|_| Error::Invalid(constants::NON_UTF8_RULE.into()))?;
            let query = std::fs::read_to_string(entry.path().join(constants::QUERY_FILE))?;
            let id =
                RuleId::try_from(id.as_str()).map_err(|error| Error::Invalid(error.to_string()))?;
            let query = RyoQlQuery::try_from(query.as_str())
                .map_err(|error| Error::Invalid(error.to_string()))?;
            if rules
                .insert(
                    id,
                    Rule {
                        query,
                        parameters: BTreeMap::new(),
                        include: vec![],
                        exclude: vec![],
                    },
                )
                .is_some()
            {
                return Err(Error::Invalid(constants::DUPLICATE_RULE.into()));
            }
        }
        if rules.is_empty() {
            return Err(Error::Invalid(constants::EMPTY_RULES.into()));
        }
        let mut sources = Vec::new();
        let mut accepted = std::collections::BTreeSet::new();
        for file in &request.files {
            relative_path(file.0.as_ref())?;
            if !accepted.insert(file.clone()) {
                return Err(Error::Invalid(
                    crate::domain::diagnostics::constants::DUPLICATE_INPUT.into(),
                ));
            }
            let source = std::fs::read_to_string(request.root.join(file.0.as_ref()))?;
            sources.push(CaseSource {
                path: file.clone(),
                text: SourceText(source.into_boxed_str()),
            });
        }
        if sources.is_empty() {
            return Err(Error::Invalid(constants::EMPTY_CORPUS.into()));
        }
        let pool = WorkerPool::from(Workers(NonZeroUsize::MIN));
        let cache = match request.state {
            crate::domain::suite::types::WorkerState::Fresh => Cache::None,
            crate::domain::suite::types::WorkerState::Retained(path) => Cache::At(path),
        };
        let report = linter::lint(
            self.languages.clone(),
            &pool,
            MemoryBudget::Machine,
            cache,
            LintRequest {
                root: request.root,
                rules,
                files: vec![],
                hosted: vec![],
                corpus: Corpus::Given(sources),
                beside: vec![],
            },
        )
        .map_err(|error| Error::Invalid(error.to_string()))?;
        if !report.unknowns.is_empty() || !report.unread.is_empty() {
            return Err(Error::Invalid(format!(
                "incomplete candidate: unknowns={:?}; unread={:?}",
                report.unknowns, report.unread
            )));
        }
        let mut diagnostics = Vec::new();
        for finding in report.findings {
            let Slot::Is(Cell::File(file)) = finding.subject else {
                return Err(Error::Invalid(constants::INVALID_SUBJECT.into()));
            };
            let spans: Vec<_> = finding
                .columns
                .iter()
                .filter_map(|column| {
                    if column.name.0.as_ref() == constants::SPAN_COLUMN {
                        if let Slot::Is(Cell::Span(span)) = &column.value {
                            return Some(*span);
                        }
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
                start: crate::domain::diagnostics::types::SourceByteOffset(span.start.0 as usize),
                end: crate::domain::diagnostics::types::SourceByteOffset(span.end.0 as usize),
            });
        }
        Ok(RunReport {
            diagnostics,
            files: accepted.into_iter().collect(),
        })
    }
}
