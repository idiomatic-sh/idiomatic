//! The types of the `compile` use case: plain data, public fields and derives, with no
//! constructor and no inherent impl. A plan holds no names: a relation is the `Relation` enum, a
//! column is a position, a step is a number, a test is an enum. Text stays only in the literals
//! the query wrote and in the headings of the rows it answers.

use ryo_values::domain::plans::types::Plan;

/// The plan the engine walks.
#[derive(Clone, Debug, PartialEq)]
pub struct CompileReport {
    /// The compiled query.
    pub plan: Plan,
    /// The query's modules as parsed, which a session hands the next compile over the same catalog.
    pub modules: ryo_values::domain::program::types::ParsedModules,
}
