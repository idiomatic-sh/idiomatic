//! Query loading, compilation and SDK startup occur inside the measured child.
use crate::domain::diagnostics::{
    errors::Error,
    types::{Diagnostic, RunReport},
};
use crate::domain::suite::service::relative_path;
use crate::domain::suite::types::WorkerRequest;
use ryo::linter::{
    self, Cache, CaseSource, Cell, Corpus, Languages, LintRequest, MemoryBudget, RelativePath,
    Rule, RuleId, Slot, SourceText, WorkerPool, Workers,
};
use ryo::query::RyoQlQuery;
use std::collections::BTreeMap;
use std::num::NonZeroUsize;

use crate::outbound::engine::interface::EnginePort;

pub(crate) struct EngineLocalAdapter {
    languages: Languages,
    trace: Option<std::path::PathBuf>,
}
impl EngineLocalAdapter {
    pub(crate) fn new(languages: Languages) -> Self {
        Self {
            languages,
            trace: None,
        }
    }
    pub(crate) fn profiled(languages: Languages, trace: std::path::PathBuf) -> Self {
        Self {
            languages,
            trace: Some(trace),
        }
    }
}
impl std::fmt::Debug for EngineLocalAdapter {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
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
                .map_err(|_| Error::Invalid("non-UTF8 rule".into()))?;
            let query = std::fs::read_to_string(entry.path().join("query.ryoql"))?;
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
                return Err(Error::Invalid("duplicate candidate rule identity".into()));
            }
        }
        if rules.is_empty() {
            return Err(Error::Invalid("no candidate queries".into()));
        }
        let mut sources = Vec::new();
        let mut accepted = std::collections::BTreeSet::new();
        for file in &request.files {
            relative_path(file)?;
            if !accepted.insert(file.clone()) {
                return Err(Error::Invalid("duplicate input path".into()));
            }
            let source = std::fs::read_to_string(request.root.join(file))?;
            sources.push(CaseSource {
                path: RelativePath(file.clone().into_boxed_str()),
                text: SourceText(source.into_boxed_str()),
            });
        }
        if sources.is_empty() {
            return Err(Error::Invalid("empty candidate corpus".into()));
        }
        let pool = WorkerPool::from(Workers(NonZeroUsize::MIN));
        let cache = request.cache.map_or(Cache::None, Cache::At);
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
                return Err(Error::Invalid("finding subject is not a file".into()));
            };
            let spans: Vec<_> = finding
                .columns
                .iter()
                .filter_map(|column| {
                    if column.name.0.as_ref() == "span" {
                        if let Slot::Is(Cell::Span(span)) = &column.value {
                            return Some(*span);
                        }
                    }
                    None
                })
                .collect();
            let [span] = spans.as_slice() else {
                return Err(Error::Invalid(
                    "finding requires one typed span column".into(),
                ));
            };
            diagnostics.push(Diagnostic {
                rule: finding.rule.to_string(),
                file: file.0.to_string(),
                start: span.start.0 as usize,
                end: span.end.0 as usize,
            });
        }
        Ok(RunReport {
            diagnostics,
            files: accepted.into_iter().collect(),
        })
    }
}
