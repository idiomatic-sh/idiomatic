//! The prepared program a caller holds: opaque, never minted by a caller.
use std::sync::Arc;
/// The implementation layers are unavailable to callers. Every caller of a prepared program is another crate, but rustc
/// refuses only what a current caller attempts: a field or method widened to `pub` would compile on every build of its
/// callers. These probes fail if the runtime widens one.
///
/// ```compile_fail,E0603
/// use ryo_execution_runtime::application::file_dag::execution::CommonOperatorEngine;
/// ```
/// The program compiler (`compile`) lives in `ryo_execution_domain`; `idiomatic.arch/library-surface` keeps it, the store
/// and the runtime's application from every crate outside the engine crates.
/// Even a valid public handle cannot expose the private execution owner.
/// Root admission and scheduling remain behind the inbound protocol.
/// ```compile_fail,E0616
/// use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
/// fn bypass(program: &PreparedExecutionProgram) { let _ = &program.inner; }
/// ```
/// ```compile_fail,E0616
/// use ryo_execution_runtime::application::file_dag::program::file_session::types::FileSession;
/// fn bypass(file: &FileSession<'_, '_>) { let _ = &file.inner; }
/// ```
/// One probe per field, so widening any one of them fails its probe.
/// ```compile_fail,E0616
/// use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
/// fn bypass(program: &PreparedExecutionProgram) { let _ = &program.kernels; }
/// ```
/// ```compile_fail,E0616
/// use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
/// fn bypass(program: &PreparedExecutionProgram) { let _ = &program.derived; }
/// ```
/// ```compile_fail,E0616
/// use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
/// fn bypass(program: &PreparedExecutionProgram) { let _ = &program.digest; }
/// ```
/// ```compile_fail,E0616
/// use ryo_execution_runtime::application::file_dag::program::file_session::types::FileSession;
/// fn bypass(file: &FileSession<'_, '_>) { let _ = &file.program; }
/// ```
/// ```compile_fail,E0616
/// use ryo_execution_runtime::application::file_dag::program::file_session::types::FileSession;
/// fn bypass(file: &FileSession<'_, '_>) { let _ = &file.file; }
/// ```
/// ```compile_fail,E0616
/// use ryo_execution_runtime::application::file_dag::program::file_session::types::FileSession;
/// fn bypass(file: &FileSession<'_, '_>) { let _ = &file.elements; }
/// ```
/// ```compile_fail,E0616
/// use ryo_execution_runtime::application::file_dag::program::file_session::types::FileSession;
/// fn bypass(file: &FileSession<'_, '_>) { let _ = &file.read; }
/// ```
/// A project session cannot be opened outside a declared root.
/// ```compile_fail,E0624
/// use ryo_execution_runtime::application::file_dag::program::project_phase::types::ProjectInput;
/// use ryo_execution_runtime::application::file_dag::program::request_execution::types::RequestExecution;
/// use ryo_language::domain::symbols::types::Symbols;
/// fn open(request: &RequestExecution, input: ProjectInput<'_>, symbols: &mut Symbols) {
///     let _ = request.open_project(input, symbols);
/// }
/// ```
/// Public prepared handles can be accepted and inspected through the inbound contract.
/// ```
/// use ryo_execution_runtime::application::file_dag::program::prepared_execution::{
///     interface::PreparedExecution, types::PreparedExecutionProgram,
/// };
/// fn count(program: &PreparedExecutionProgram) -> usize { program.rule_count().0 }
/// ```
/// They cannot be minted by a caller.
/// ```compile_fail,E0451
/// use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
/// let _ = PreparedExecutionProgram { inner: panic!(), kernels: panic!(), derived: panic!(), digest: panic!() };
/// ```
/// ```compile_fail,E0451
/// use ryo_execution_runtime::application::file_dag::program::file_session::types::FileSession;
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
