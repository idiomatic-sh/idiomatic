//! The types of the `typecheck` use case: what the checks leave of a query that passed them.

use super::errors::TypecheckError;

/// What the checks make of a query that passed them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Checked {
    /// The headings of the answer the query emits, and how many steps the checks built resolving every name.
    Lowered {
        /// The names of the columns of the emitted rows.
        headings: Vec<Box<str>>,
        /// How many steps the checks built.
        steps: usize,
    },
    /// A query the checks cannot follow to its end: a select keeps a column a function computes, which the checks' scope cannot hold. The
    /// refusal they would give is held, and the plan that runs judges the query.
    Deferred(TypecheckError),
}
