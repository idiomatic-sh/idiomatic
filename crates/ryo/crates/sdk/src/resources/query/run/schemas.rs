//! The shape of the data this entry does not control: the compiler's headings for a query, and the failures of a run as the caller reads them.

use std::sync::Arc;

use super::errors::RunError;
use super::types::Heading;
use crate::domain::query::types::RyoQlQuery;
use crate::resources::linter::LintError;

/// The headings of the answer of a query: what its `emit` names. A query that is not correct is refused as the engine's query-language
/// port says it.
///
/// # Errors
///
/// `Query` for a query that is not correct.
pub(super) fn headings(query: &RyoQlQuery) -> Result<Vec<Heading>, RunError> {
    use ryo_query_language::inbound::use_cases::typecheck::{
        interface::TypecheckUseCase as _, service::refused, types::Checked,
    };
    let typecheck = ryo_query_language::inbound::use_cases::typecheck::use_case::TypecheckUseCaseImpl::new(
        Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
        Arc::new(ryo_query_language::outbound::catalog::factory::language()),
    );
    let refusal = |error| RunError::Query { reason: refused(error).to_string() };
    match typecheck
        .check(ryo_values::domain::query_requests::types::QueryRequest {
            text: query.clone().into_bundle(),
            hosted: Vec::new(),
            modules: ryo_values::domain::program::types::ParsedModules::default(),
            observations: Vec::new(),
        })
        .map_err(refusal)?
    {
        Checked::Plan(plan) => Ok(plan.emit.headings.into_iter().map(|each| Heading(each.0)).collect()),
        Checked::Deferred(error) => Err(refusal(error)),
    }
}

/// A failure of the one query `id` among several, said naming the query by its place; as it is when the run has the one.
pub(super) fn among(error: RunError, id: &str, several: bool) -> RunError {
    match error {
        RunError::Query { reason } if several => RunError::Query { reason: format!("rule `{id}`: {reason}") },
        other => other,
    }
}

/// The caller's failure for the linter's: a wrong query is the query's own, naming it by its place when the run has several.
pub(super) fn error(error: LintError, several: bool) -> RunError {
    match error {
        LintError::Rule { id, error } => among(RunError::Query { reason: error.to_string() }, id.as_ref(), several),
        LintError::MissingParameter { id } => among(
            RunError::Query { reason: "the query has no value for a parameter it declares".to_owned() },
            id.as_ref(),
            several,
        ),
        LintError::Io { reason, .. } => RunError::Corpus { reason },
        LintError::PathNotText { path } => RunError::PathNotText { path },
        LintError::TooLarge { size, budget } => RunError::TooLarge { size, budget },
        LintError::MemoryBudgetExceeded { used, budget, plans } => RunError::MemoryBudgetExceeded {
            used,
            budget,
            plans: plans.iter().map(ToString::to_string).collect(),
        },
        LintError::Execution { reason } | LintError::ObservationExecution { reason } => RunError::Evaluate { reason },
        other => RunError::Language { reason: other.to_string() },
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
