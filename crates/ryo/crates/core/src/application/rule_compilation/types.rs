//! Bounded session plans and one invocation's compiled/bound answer.
use crate::domain::table_model::types::ParameterBinding;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::program::types::QueryBundle;
use ryo_values::domain::runs::types::RuleId;
use std::sync::{Arc, Mutex};

/// The plan compiled so far for each rule id, shared by every use case built over them: a program that lints many projects with the
/// same rules compiles each rule once. A rule id keeps one plan: the one of its latest query; past `COMPILED_PLANS` the plan that
/// has been kept longest is dropped, whatever the request in progress names: a run holds the plans it compiled or found, so
/// dropping one never changes an answer.
#[derive(Clone, Debug, Default)]
pub struct CompiledPlans(pub Arc<Mutex<PlanCache>>);

/// The plans compiled so far, and the order they were first kept in.
#[derive(Debug, Default)]
pub struct PlanCache {
    /// The plan of each rule id.
    pub plans: std::collections::HashMap<RuleId, CachedPlan>,
    /// The rule ids of `plans`, the one kept longest first, each once.
    pub order: std::collections::VecDeque<RuleId>,
}

/// The plan a rule's query compiled to, kept with the query it was compiled from: a rule whose query changed is compiled again.
#[derive(Clone, Debug)]
pub struct CachedPlan {
    /// The RyoQL text the plan was compiled from.
    pub query: QueryBundle,
    /// The relations the host declared when it was: a plan that names one is another plan when they are others.
    pub hosted: Vec<ryo_values::domain::hosted::types::HostedRelation>,
    /// The plan.
    pub plan: Plan,
}

/// The plans of the rules, and what each binds its parameters to.
#[derive(Clone, Debug)]
pub(crate) struct Compiled {
    /// The plans, in the order of the rules.
    pub plans: Vec<Plan>,
    /// What each rule binds its parameters to, in the order of the plan's parameters.
    pub bindings: Vec<Vec<ParameterBinding>>,
}
