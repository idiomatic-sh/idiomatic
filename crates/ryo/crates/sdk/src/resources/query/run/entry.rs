//! `query run`: composes, calls the service once.

use std::collections::BTreeMap;

use super::errors::RunError;
use super::types::{Heading, Row, RowCount, RunReport, RunRequest};
use crate::domain::capacity::types::{MemoryBudget, WorkerPool};
use crate::domain::cell::types::{Cell, HostNumber, Slot};
use crate::domain::language::types::Languages;
use crate::domain::place::types::Place;
use crate::resources::linter as sdk;

/// The id the query has as the one rule of the lint it runs as.
const QUERY_ID: &str = "query";

/// `query run`: explores one query over a repository, with the languages the program linked. The rows it answers are kept in the
/// report, headings included. The engine bounds its own memory: `memory` is not read.
///
/// # Errors
///
/// `Query` for a wrong query; `PathNotText` for a root that is not text; a port's failure otherwise.
pub fn run(
    languages: Languages,
    pool: &WorkerPool,
    memory: MemoryBudget,
    request: RunRequest,
) -> Result<RunReport, RunError> {
    let _ = memory;
    if request.root.to_str().is_none() {
        return Err(RunError::PathNotText { path: request.root });
    }
    let mut columns = super::schemas::headings(&request.query)?;
    columns.push(Heading("line".into()));
    columns.push(Heading("column".into()));
    let rule = sdk::Rule {
        observations: Vec::new(),
        query: request.query,
        applicability: sdk::RuleApplicability::Always,
        parameters: BTreeMap::new(),
        include: Vec::new(),
        exclude: Vec::new(),
    };
    let id = sdk::RuleId::try_from(QUERY_ID).map_err(|error| RunError::Language { reason: error.to_string() })?;
    let lint = sdk::LintRequest {
        source_language: sdk::SourceLanguage::Recognized,
        root: request.root,
        project_search_scope: Default::default(),
        given_project_inventory: Default::default(),
        rules: BTreeMap::from([(id, rule)]),
        files: Vec::new(),
        hosted: Vec::new(),
        corpus: sdk::Corpus::Walked,
        beside: Vec::new(),
    };
    let linter = sdk::Linter::new(languages, pool.clone(), sdk::Cache::None);
    let report = linter
        .lint_rows(lint, usize::try_from(request.limit.0).unwrap_or(usize::MAX))
        .map_err(super::schemas::error)?;
    let rows: Vec<Row> = report
        .findings
        .iter()
        .map(|finding| {
            let by_name: BTreeMap<&str, &Slot> =
                finding.columns.iter().map(|column| (column.name.0.as_ref(), &column.value)).collect();
            let mut cells: Vec<Slot> = columns
                .iter()
                .take(columns.len().saturating_sub(2))
                .map(|heading| by_name.get(heading.0.as_ref()).map_or(Slot::Nothing, |slot| (*slot).clone()))
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
