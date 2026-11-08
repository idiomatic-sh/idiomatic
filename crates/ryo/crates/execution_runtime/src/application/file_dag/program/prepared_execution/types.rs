//! The prepared program a caller holds: opaque, never minted by a caller.
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
    pub(crate) derived: ryo_execution_store::application::file_dag::derived::owner::types::DerivedAccess,
    pub(crate) inner:
        Arc<crate::application::file_dag::program::preparation::types::PreparedProgram>,
    /// What names the content of packed words, for every request of the program.
    pub(crate) digest: Arc<dyn crate::outbound::digest::interface::DigestPort>,
}
