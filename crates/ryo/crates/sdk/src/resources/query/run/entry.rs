//! `query run`: composes, calls the service once.

use ryo_core::inbound::use_cases::lint::interface::LintUseCase as _;
use ryo_core::inbound::use_cases::lint::types::LintRequest as CoreRequest;

use super::errors::RunError;
use super::schemas;
use super::types::{RunReport, RunRequest};
use crate::domain::capacity::types::{MemoryBudget, WorkerPool};
use crate::domain::language::types::Languages;

/// `query run`: explores one query over a repository, with the languages the program linked. The rows it answers are kept in the
/// report, headings included.
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
    let budget = schemas::budget(memory);
    let core: CoreRequest = schemas::request(request)?;
    let rows = super::composition::build_rows();
    let report = super::composition::build(
        languages.0,
        rows.clone(),
        core.session.root.clone(),
        budget,
        pool.clone(),
    )
    .lint(core)
    .map_err(schemas::error)?;
    Ok(schemas::report(rows.take(), report))
}
