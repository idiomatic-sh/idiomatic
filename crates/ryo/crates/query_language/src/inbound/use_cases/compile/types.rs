//! The types of the `compile` use case: plain data, public fields and derives, with no
//! constructor and no inherent impl. A plan holds no names: a relation is the `Relation` enum, a
//! column is a position, a step is a number, a test is an enum. Text stays only in the literals
//! the query wrote and in the headings of the rows it answers.

use ryo_values::domain::query_plans::types::Plan;

/// The plan the engine walks.
#[derive(Clone, Debug, PartialEq)]
pub struct CompileReport {
    /// The compiled query.
    pub plan: Plan,
    /// The query's modules as parsed, which a session hands the next compile over the same catalog.
    pub modules: ryo_values::domain::program::types::ParsedModules,
}

/// What a compile keeps of the modules it was last over, for the next compile over the same parsed modules: their
/// fragments' checks, made once however many rules import them, and the expansions of them lowered so far.
#[derive(Clone, Debug)]
pub(crate) struct CompiledModules {
    /// The parsed modules the rest is of: never read for another catalog's.
    pub modules: ryo_values::domain::program::types::ParsedModules,
    /// The checks of their fragments.
    pub checks: std::sync::Arc<crate::domain::checking::types::FragmentChecks>,
    /// The expansions of their fragments lowered so far.
    pub expansions: std::sync::Arc<crate::domain::splicing::types::LoweredExpansions>,
}
