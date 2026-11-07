//! Bounded session plans and one invocation's compiled/bound answer.
use crate::domain::prepared_request::types::ObservationIdentity;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::query_tokens::types::QueryBundle;
use ryo_values::domain::runs::types::RuleId;

/// The plans compiled so far, and the order they were first kept in.
#[derive(Debug, Default)]
pub(crate) struct PlanCache {
    /// The plan of each rule id.
    pub plans: std::collections::HashMap<RuleId, CachedPlan>,
    /// The rule ids of `plans`, the one kept longest first, each once.
    pub order: std::collections::VecDeque<RuleId>,
    /// The module catalog the session parsed last, the one the latest rule over modules named.
    pub catalog: ryo_values::domain::query_tokens::types::ModuleCatalog,
    /// The modules of `catalog` as the query language parsed them, so the rules over one catalog parse it once.
    pub parsed: ryo_values::domain::program::types::ParsedModules,
}

/// The plan a rule's query compiled to, kept with the query it was compiled from: a rule whose query changed is compiled again.
#[derive(Clone, Debug)]
pub(crate) struct CachedPlan {
    /// The RyoQL text the plan was compiled from.
    pub query: QueryBundle,
    /// The relations the host declared when it was: a plan that names one is another plan when they are others.
    pub hosted: Vec<ryo_values::domain::hosted::types::HostedRelation>,
    /// Explicit binding declarations and inferred schema/phase are part of compilation identity.
    pub observation_bindings: Vec<ryo_values::domain::observations::types::ObservationBinding>,
    /// Typed producer inputs used by this cached query plan.
    pub observation_inputs: Vec<ryo_values::domain::observations::types::BoundObservationInput>,
    /// The plan with every declared observation root retained, never request-pruned.
    pub plan: Plan,
}

/// Retained rule identities and the required semantic layout for named frontier bits.
pub struct PreparedRuleIdentities {
    pub rules: Vec<ryo_values::domain::stores::types::StoredRule>,
    pub observations: ObservationIdentity,
}
