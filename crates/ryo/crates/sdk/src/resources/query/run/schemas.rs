//! The shape of the data this entry does not control: the compiler's headings for a query, and the failures of a run as the caller reads them.

use ryo_execution::outbound::query_language::errors::QueryLanguageError;
use ryo_execution::outbound::query_language::interface::QueryLanguagePort as _;
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
    let compile = Arc::new(ryo_query_language::inbound::use_cases::compile::use_case::CompileUseCaseImpl::new(
        Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
        Arc::new(ryo_query_language::outbound::catalog::factory::language()),
    ));
    let typecheck = Arc::new(ryo_query_language::inbound::use_cases::typecheck::use_case::TypecheckUseCaseImpl::new(
        Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
        Arc::new(ryo_query_language::outbound::catalog::factory::language()),
    ));
    let port = ryo_execution::outbound::query_language::factory::use_case(compile, typecheck);
    let bundle = query.clone().into_bundle();
    let plan = port
        .compile(&bundle, &[], &[])
        .map_err(|QueryLanguageError::Refused { error }| RunError::Query { reason: error.to_string() })?;
    Ok(plan.emit.headings.into_iter().map(|each| Heading(each.0)).collect())
}

/// The caller's failure for the linter's.
pub(super) fn error(error: LintError) -> RunError {
    match error {
        LintError::Rule { error, .. } => RunError::Query { reason: error.to_string() },
        LintError::MissingParameter { .. } => RunError::Query {
            reason: "the query has no value for a parameter it declares".to_owned(),
        },
        LintError::Io { reason, .. } => RunError::Corpus { reason },
        LintError::PathNotText { path } => RunError::PathNotText { path },
        LintError::TooLarge { size, budget } => RunError::TooLarge { size, budget },
        LintError::MemoryBudgetExceeded { used, budget, .. } => RunError::MemoryBudgetExceeded { used, budget },
        LintError::Execution { reason } | LintError::ObservationExecution { reason } => RunError::Evaluate { reason },
        other => RunError::Language { reason: other.to_string() },
    }
}
