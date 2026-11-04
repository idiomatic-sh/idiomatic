//! The types of the `compile` use case: plain data, public fields and derives, with no
//! constructor and no inherent impl. A plan holds no names: a relation is the `Relation` enum, a
//! column is a position, a step is a number, a test is an enum. Text stays only in the literals
//! the query wrote and in the headings of the rows it answers.

use ryo_values::domain::hosted::types::HostedRelation;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::program::types::QueryBundle;

/// The RyoQL text to compile, and the relations the host declares beside ryo's own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompileRequest {
    /// The query: text that entered the engine.
    pub text: QueryBundle,
    /// The relations the host declares, which a query names as it names ryo's: their rows come with the run.
    pub hosted: Vec<HostedRelation>,
    /// Resolved named producer contracts from the request graph's declaration pass.
    pub observations: Vec<ryo_values::domain::observations::types::BoundObservationSchema>,
}

/// The plan the engine walks.
#[derive(Clone, Debug, PartialEq)]
pub struct CompileReport {
    /// The compiled query.
    pub plan: Plan,
}
