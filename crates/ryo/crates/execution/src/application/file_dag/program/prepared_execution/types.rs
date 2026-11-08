//! The prepared program a caller holds: opaque, never minted by a caller.
use std::sync::Arc;
/// The implementation layers are unavailable to callers.
///
/// ```compile_fail,E0603
/// use ryo_execution::application::file_dag::execution::CommonOperatorEngine;
/// ```
/// ```compile_fail,E0603
/// use ryo_execution::domain::executable_program::service::compile;
/// ```
/// The store's owners (`ryo_execution_store::application::file_dag::relations::RelationOwner` and the rest) are
/// public to the engine crates; `idiomatic.arch/library-surface` keeps every other crate to the store's derived port.
/// There is no production strategy selector: the removed public kernel-strategy
/// choice and the tree-step choice no longer resolve, and no execution API names them.
/// ```compile_fail,E0432
/// use ryo_execution::inbound::use_cases::execution::types::KernelStrategy;
/// ```
/// ```compile_fail,E0432
/// use ryo_execution::inbound::use_cases::execution::types::TreeSteps;
/// ```
/// Even a valid public handle cannot expose the private execution owner.
/// Root admission and scheduling remain behind the inbound protocol.
/// ```compile_fail,E0616
/// use ryo_execution::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
/// fn bypass(program: &PreparedExecutionProgram) { let _ = &program.inner; }
/// ```
/// ```compile_fail,E0616
/// use ryo_execution::application::file_dag::program::file_session::types::FileSession;
/// fn bypass(file: &FileSession<'_, '_>) { let _ = &file.inner; }
/// ```
/// A project session cannot be named or opened outside a declared root: the removed
/// session type, its trait and the request's open method no longer resolve.
/// ```compile_fail,E0432
/// use ryo_execution::inbound::use_cases::execution::types::ProjectSession;
/// ```
/// ```compile_fail,E0432
/// use ryo_execution::inbound::use_cases::execution::interface::ProjectExecution;
/// ```
/// ```compile_fail,E0624
/// use ryo_execution::application::file_dag::program::project_phase::types::ProjectInput;
/// use ryo_execution::application::file_dag::program::request_execution::types::RequestExecution;
/// use ryo_language::domain::symbols::types::Symbols;
/// fn open(request: &RequestExecution, input: ProjectInput<'_>, symbols: &mut Symbols) {
///     let _ = request.open_project(input, symbols);
/// }
/// ```
/// Public prepared handles can be accepted and inspected through the inbound contract.
/// ```
/// use ryo_execution::application::file_dag::program::prepared_execution::{
///     interface::PreparedExecution, types::PreparedExecutionProgram,
/// };
/// fn count(program: &PreparedExecutionProgram) -> usize { program.rule_count().0 }
/// ```
/// They cannot be minted by a caller.
/// ```compile_fail,E0451
/// use ryo_execution::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
/// let _ = PreparedExecutionProgram { inner: panic!(), kernels: panic!(), derived: panic!(), digest: panic!() };
/// ```
/// ```compile_fail,E0451
/// use ryo_execution::application::file_dag::program::file_session::types::FileSession;
/// let _ = FileSession {
///     inner: panic!(), program: panic!(), file: panic!(),
///     elements: panic!(), read: panic!(),
/// };
/// ```
pub struct PreparedExecutionProgram {
    pub(crate) kernels: Arc<dyn crate::application::file_dag::executor::PrimitiveExecutor + Send>,
    pub(crate) derived: ryo_execution_store::application::file_dag::derived::owner::types::DerivedAccess,
    pub(crate) inner:
        Arc<crate::application::file_dag::program::preparation::types::PreparedProgram>,
    /// What names the content of packed words, for every request of the program.
    pub(crate) digest: Arc<dyn crate::outbound::digest::interface::DigestPort>,
}
