//! What `prepare` takes: the derived views it prepares over, and a request's plans, rules, digests, hosted relations
//! and observation identity.
use crate::application::rule_compilation::service::CompiledRules;
use ryo_execution_domain::domain::prepared_request::types::ObservationIdentity;

/// Where the plans of a request come from.
#[derive(Debug)]
pub enum PlanSource {
    /// Plans and their bindings as the caller authored them.
    Authored {
        /// The plans, one for each rule.
        plans: Vec<ryo_values::domain::query_plans::types::Plan>,
        /// The parameter bindings of each plan.
        bindings: Vec<Vec<ryo_values::domain::query_bindings::types::ParameterBinding>>,
    },
    /// Rules the `compile` use case compiled.
    Compiled(CompiledRules),
}

/// What a caller asks to have prepared.
#[derive(Debug)]
pub struct PrepareRequest {
    /// The plans of the request.
    pub plans: PlanSource,
    /// The rules the plans belong to.
    pub rules: Vec<ryo_values::domain::rule_requests::types::RuleSpec>,
    /// The digest of each rule.
    pub digests: Vec<ryo_values::domain::stored_facts::types::Digest>,
    /// The relations the host provides.
    pub hosted: Vec<ryo_values::domain::hosted::types::HostRelation>,
    /// The identity of the observations the request names.
    pub observations: ObservationIdentity,
}
