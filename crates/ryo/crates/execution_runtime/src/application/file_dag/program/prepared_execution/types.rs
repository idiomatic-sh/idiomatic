//! The prepared program a caller holds: opaque, never minted by a caller.
use ryo_execution_domain::domain::observation_schedule::types::ObservedFrontierRoot;
use ryo_values::domain::records::types::ObservationReadinessLayout;
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stores::types::Reuse;
use std::sync::Arc;
/// The implementation layers are unavailable to callers. Each handle holds its members in one crate-private parts
/// struct, and the workspace denies `private_interfaces`: widening `parts` to `pub` exposes a crate-private type, so the
/// runtime's own build refuses it. The same lint holds `RequestExecution::open_project`, whose return type is crate-private.
///
/// ```compile_fail,E0603
/// use ryo_execution_runtime::application::file_dag::execution::CommonOperatorEngine;
/// ```
/// The program compiler (`compile`) lives in `ryo_execution_domain`; `idiomatic.arch/library-surface` keeps it, the store
/// and the runtime's application from every crate outside the engine crates.
/// Public prepared handles can be accepted and inspected through the inbound contract.
/// ```
/// use ryo_execution_runtime::application::file_dag::program::prepared_execution::{
///     interface::PreparedExecution, types::PreparedExecutionProgram,
/// };
/// fn count(program: &PreparedExecutionProgram) -> usize { program.rule_count().0 }
/// ```
pub struct PreparedExecutionProgram {
    pub(crate) parts: ProgramParts,
}
/// The members of a prepared program, reachable only inside the runtime.
pub(crate) struct ProgramParts {
    pub(crate) kernels: Arc<dyn crate::application::file_dag::executor::PrimitiveExecutor + Send>,
    pub(crate) derived:
        ryo_execution_store::application::file_dag::derived::owner::service::DerivedAccess,
    pub(crate) inner:
        Arc<crate::application::file_dag::program::preparation::types::PreparedProgram>,
    /// What names the content of packed words, for every request of the program.
    pub(crate) digest: Arc<dyn crate::outbound::digest::interface::DigestPort>,
}
/// Whether a rule is wanted by the request, directly or through a named observation.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::Demand;
/// assert_ne!(Demand::Demanded, Demand::Undemanded);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Demand {
    Demanded,
    Undemanded,
}
/// Whether a rule declares or consumes named observations.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::ObservationContract;
/// assert_ne!(ObservationContract::Declared, ObservationContract::Absent);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationContract {
    Declared,
    Absent,
}
/// The readiness layout of the named observations and the frontiers that feed them.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::coverage::types::RuleIndex;
/// use ryo_execution_domain::domain::observation_schedule::types::ObservedFrontierRoot;
/// use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::NamedLayout;
/// use ryo_values::domain::plan_parts::types::InputNumber;
/// use ryo_values::domain::records::types::ObservationIndex;
/// use ryo_values::domain::records::types::ObservationReadinessLayout;
/// use ryo_values::domain::stored_facts::types::Digest;
/// let frontiers = [ObservedFrontierRoot { producer: RuleIndex(0), output: ObservationIndex(0), frontier: InputNumber(0) }];
/// let named = NamedLayout { layout: ObservationReadinessLayout(Digest([0; 16])), frontiers: &frontiers };
/// assert_eq!(named.frontiers.len(), 1);
/// ```
pub struct NamedLayout<'a> {
    pub layout: ObservationReadinessLayout,
    pub frontiers: &'a [ObservedFrontierRoot],
}
/// The rules a changed selection reaches and the producer frontiers they read.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::ReplayClosure;
/// use ryo_values::domain::stored_facts::types::Bucket;
/// use ryo_values::domain::stores::types::Reuse;
/// let closure = ReplayClosure { reuse: Reuse::Whole, buckets: vec![Bucket(0)] };
/// assert_eq!(closure, ReplayClosure { reuse: Reuse::Whole, buckets: vec![Bucket(0)] });
/// ```
#[derive(Debug, PartialEq)]
pub struct ReplayClosure {
    pub reuse: Reuse,
    pub buckets: Vec<Bucket>,
}
