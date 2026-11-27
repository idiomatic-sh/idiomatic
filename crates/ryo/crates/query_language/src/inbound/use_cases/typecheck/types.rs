//! The types of the `typecheck` use case: what the checks leave of a query that passed them.

use ryo_values::domain::query_plans::types::Plan;

use super::errors::TypecheckError;

/// What the checks make of a query that passed them.
#[derive(Clone, Debug, PartialEq)]
pub enum Checked {
    /// The plan the checks built as they resolved every name: what the kind check and the answer's headings read.
    Plan(Plan),
    /// A query the checks cannot follow to its end: a select keeps a column a function computes, which the checks' plan cannot hold. The
    /// refusal they would give is held, and the plan that runs judges the query.
    Deferred(TypecheckError),
}
