//! What `prepare` takes: the derived views it prepares over, and a request's plans, rules, digests, hosted relations
//! and observation identity.
use crate::application::rule_compilation::service::CompiledRules;
use ryo_execution_domain::domain::prepared_request::types::ObservationIdentity;

/// Where the plans of a request come from.
#[derive(Debug)]
pub enum PlanSource {
    /// Plans and their bindings as the caller authored them.
    Authored {
        plans: Vec<ryo_values::domain::plans::types::Plan>,
        bindings: Vec<Vec<ryo_values::domain::query_bindings::types::ParameterBinding>>,
    },
    /// Rules the `compile` use case compiled.
    Compiled(CompiledRules),
}

/// What a caller asks to have prepared.
#[derive(Debug)]
pub struct PrepareRequest {
    pub plans: PlanSource,
    pub rules: Vec<ryo_values::domain::rule_requests::types::RuleSpec>,
    pub digests: Vec<ryo_values::domain::stored_facts::types::Digest>,
    pub hosted: Vec<ryo_values::domain::hosted::types::HostRelation>,
    pub observations: ObservationIdentity,
}
