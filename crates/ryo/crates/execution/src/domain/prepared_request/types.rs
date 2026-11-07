//! Pure data crossing request preparation; no executable authority is exposed.
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::query_bindings::types::ParameterBinding;
/// The plans of the rules, and what each binds its parameters to.
#[derive(Clone, Debug)]
pub struct Compiled {
    /// A once-resolved traversal over stable original rule indices.
    pub schedule: crate::domain::file_program::types::RuleSchedule,
    /// Names resolved once to real producer roots; empty on ordinary requests.
    pub observations: Vec<Vec<crate::domain::file_program::types::ResolvedObservationBinding>>,
    /// The plans, in the order of the rules.
    pub plans: Vec<Plan>,
    /// What each rule binds its parameters to, in the order of the plan's parameters.
    pub bindings: Vec<Vec<ParameterBinding>>,
}

#[derive(Clone, Copy, Debug)]
pub enum ObservationIdentity {
    Independent,
    Named(ryo_values::domain::records::types::ObservationReadinessLayout),
}
