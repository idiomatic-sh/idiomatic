//! What `prepare` takes: the derived views it prepares over, and a request's plans, rules, digests, hosted relations
//! and observation identity.
use crate::application::derived::service::DerivedPreparation;
use crate::application::rule_compilation::service::CompiledRules;
use ryo_execution_domain::domain::prepared_request::types::ObservationIdentity;

/// Where the plans of a request come from.
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
pub struct PrepareRequest {
    pub plans: PlanSource,
    pub rules: Vec<ryo_values::domain::rule_requests::types::RuleSpec>,
    pub digests: Vec<ryo_values::domain::stored_facts::types::Digest>,
    pub hosted: Vec<ryo_values::domain::hosted::types::HostRelation>,
    pub observations: ObservationIdentity,
}

/// The derived views `prepare` prepares over: composed once at the edge ([`crate::derived`]) and cloned into each prepare
/// use case. Execution's own handle, opaque to its callers: it lends no access and acquires no view, and no caller
/// outside execution reaches the preparation it holds.
///
/// ```compile_fail,E0616
/// let views = ryo_execution::derived();
/// let _preparation = views.0;
/// ```
///
/// # Examples
///
/// ```
/// use ryo_execution::inbound::use_cases::prepare::types::DerivedViews;
///
/// let views: DerivedViews = ryo_execution::derived();
/// let shared = views.clone();
/// ```
#[derive(Clone, Debug)]
pub struct DerivedViews(pub(crate) DerivedPreparation);

/// Whatever execution's derived preparation is built from (itself, or a derived port) builds the views over it.
impl<T> From<T> for DerivedViews
where
    DerivedPreparation: From<T>,
{
    fn from(preparation: T) -> Self {
        Self(DerivedPreparation::from(preparation))
    }
}
