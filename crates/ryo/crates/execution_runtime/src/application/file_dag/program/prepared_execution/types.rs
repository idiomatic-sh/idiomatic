//! The prepared program a caller holds: opaque, never minted by a caller.
use std::sync::Arc;
/// The implementation layers are unavailable to callers: the program's fields, its kernels and the file DAG's private
/// modules are crate-private to `ryo_execution_runtime`, and every caller of a prepared program is another crate, so rustc
/// refuses a read (E0616), a literal (E0451) or a private path (E0603) on every build of its callers.
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
    pub(crate) kernels: Arc<dyn crate::application::file_dag::executor::PrimitiveExecutor + Send>,
    pub(crate) derived: ryo_execution_store::application::file_dag::derived::owner::types::DerivedAccess,
    pub(crate) inner:
        Arc<crate::application::file_dag::program::preparation::types::PreparedProgram>,
    /// What names the content of packed words, for every request of the program.
    pub(crate) digest: Arc<dyn crate::outbound::digest::interface::DigestPort>,
}
