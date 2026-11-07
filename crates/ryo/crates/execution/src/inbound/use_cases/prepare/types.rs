//! The request `prepare` takes: its queries, rules, digests, hosted relations and observation identity.
use crate::application::rule_compilation::service::CompiledQueries;
use crate::domain::prepared_request::types::ObservationIdentity;

/// Where the plans of a request come from.
pub enum PlanSource {
    /// Plans and their bindings as the caller authored them.
    Authored {
        plans: Vec<ryo_values::domain::plans::types::Plan>,
        bindings: Vec<Vec<ryo_values::domain::query_bindings::types::ParameterBinding>>,
    },
    /// Queries a compilation already produced.
    Compiled(CompiledQueries),
}

/// What a caller asks to have prepared.
pub struct PrepareRequest {
    pub plans: PlanSource,
    pub rules: Vec<ryo_values::domain::rule_requests::types::RuleSpec>,
    pub digests: Vec<ryo_values::domain::stored_facts::types::Digest>,
    pub hosted: Vec<ryo_values::domain::hosted::types::HostRelation>,
    pub observations: ObservationIdentity,
}
