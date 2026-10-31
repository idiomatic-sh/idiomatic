//! Query loading, compilation and SDK startup occur inside the measured child.
use crate::domain::diagnostics::{
    errors::Error,
    types::{Diagnostic, RunReport},
};
use crate::domain::suite::types::WorkerRequest;
use crate::outbound::suite::adapters::local::adapter::relative_path;
use ryo::linter::{
    self, Cache, Cell, Corpus, Languages, LintRequest, MemoryBudget, RelativePath, Rule, RuleId,
    Slot, WorkerPool, Workers,
};
use ryo::query::RyoQlQuery;
use std::collections::BTreeMap;
use std::num::NonZeroUsize;

pub fn run_ryo(languages: Languages, request: WorkerRequest) -> Result<RunReport, Error> {
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
        rules.insert(
            id,
            Rule {
                query,
                parameters: BTreeMap::new(),
                include: vec![],
                exclude: vec![],
            },
        );
    }
    if rules.is_empty() {
        return Err(Error::Invalid("no candidate queries".into()));
    }
    for file in &request.files {
        relative_path(file)?;
    }
    let pool = WorkerPool::from(Workers(NonZeroUsize::MIN));
    let cache = request.cache.map_or(Cache::None, Cache::At);
    let report = linter::lint(
        languages,
        &pool,
        MemoryBudget::Machine,
        cache,
        LintRequest {
            root: request.root,
            rules,
            files: vec![],
            hosted: vec![],
            corpus: Corpus::Listed(
                request
                    .files
                    .iter()
                    .map(|path| RelativePath(path.clone().into_boxed_str()))
                    .collect(),
            ),
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
        files: request.files,
    })
}

pub struct EngineLocalAdapter {
    pub languages: Languages,
}
impl crate::outbound::engine::interface::EnginePort for EngineLocalAdapter {
    fn run(&self, request: WorkerRequest) -> Result<RunReport, Error> {
        run_ryo(self.languages.clone(), request)
    }
}
