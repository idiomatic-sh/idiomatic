//! `query run`: lists the repository once, answers the queries as the rows of one request over the executor, and shapes the answer.

use std::collections::BTreeMap;

use super::errors::RunError;
use super::types::{Heading, Row, RowCount, RunReport, RunRequest};
use crate::domain::capacity::types::{MemoryBudget, WorkerPool};
use crate::domain::cell::types::{Cell, HostNumber, HostText, Slot};
use crate::domain::language::types::Languages;
use crate::domain::place::types::Place;
use crate::resources::linter as sdk;

/// `query run`: explores queries over a repository, with the languages the program linked. The rows it answers are kept in the
/// report, headings included. The files are listed once and held against `memory` before any is read.
///
/// # Errors
///
/// `Query` for a wrong query, naming it by its place when there are several; `PathNotText` for a root that is not text; `Corpus` when
/// the repository cannot be listed; `TooLarge` or `MemoryBudgetExceeded` when it does not fit `memory`; a port's failure otherwise.
pub fn run(
    languages: Languages,
    pool: &WorkerPool,
    memory: MemoryBudget,
    request: RunRequest,
) -> Result<RunReport, RunError> {
    if request.root.to_str().is_none() {
        return Err(RunError::PathNotText { path: request.root });
    }
    let several = request.queries.len() > 1;
    let mut headings: Vec<Vec<Heading>> = Vec::new();
    let mut rules = BTreeMap::new();
    for (place, query) in request.queries.into_iter().enumerate() {
        let id = (place + 1).to_string();
        headings.push(super::schemas::headings(&query).map_err(|error| super::schemas::among(error, &id, several))?);
        let rule = sdk::Rule {
            observations: Vec::new(),
            query,
            applicability: sdk::RuleApplicability::Always,
            parameters: BTreeMap::new(),
            include: Vec::new(),
            exclude: Vec::new(),
        };
        let id = sdk::RuleId::try_from(id.as_str()).map_err(|error| RunError::Language { reason: error.to_string() })?;
        let _prior = rules.insert(id, rule);
    }
    // The columns: with several queries the query's place first, then every query's columns, each once, then the place of the row.
    let mut columns: Vec<Heading> = Vec::new();
    if several {
        columns.push(Heading("rule".into()));
    }
    for each in headings.iter().flatten() {
        if !columns.contains(each) {
            columns.push(each.clone());
        }
    }
    let kept = columns.len();
    columns.push(Heading("line".into()));
    columns.push(Heading("column".into()));
    let lint = sdk::LintRequest {
        source_language: sdk::SourceLanguage::Recognized,
        root: request.root,
        project_search_scope: Default::default(),
        rules,
        files: Vec::new(),
        hosted: Vec::new(),
        corpus: sdk::Corpus::Walked,
        beside: Vec::new(),
    };
    let linter = sdk::Linter::new(languages, pool.clone(), sdk::Cache::None);
    let report = sdk::lint::rows::answer(&linter, lint, usize::try_from(request.limit.0).unwrap_or(usize::MAX), memory)
        .map_err(|error| super::schemas::error(error, several))?;
    let rows: Vec<Row> = report
        .findings
        .iter()
        .map(|finding| {
            let by_name: BTreeMap<&str, &Slot> =
                finding.columns.iter().map(|column| (column.name.0.as_ref(), &column.value)).collect();
            let mut cells: Vec<Slot> = columns
                .iter()
                .take(kept)
                .enumerate()
                .map(|(at, heading)| {
                    if several && at == 0 {
                        Slot::Is(Cell::Text(HostText(finding.rule.to_string().into())))
                    } else {
                        by_name.get(heading.0.as_ref()).map_or(Slot::Nothing, |slot| (*slot).clone())
                    }
                })
                .collect();
            match finding.place {
                Place::At { line, column } => {
                    cells.push(Slot::Is(Cell::Number(HostNumber(i64::from(line.0)))));
                    cells.push(Slot::Is(Cell::Number(HostNumber(i64::from(column.0)))));
                }
                _ => cells.extend([Slot::Nothing, Slot::Nothing]),
            }
            Row(cells)
        })
        .collect();
    Ok(RunReport {
        columns,
        shown: RowCount(rows.len() as u64),
        total: RowCount(report.rows_total as u64),
        rows,
        unread: report.unread,
    })
}
