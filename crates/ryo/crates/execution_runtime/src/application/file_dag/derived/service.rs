//! The `derived` application operation: a request's program prepared over the derived access, and the file preparation
//! of the derived owner.
use super::program::service::ProgramDerived;
use crate::application::file_dag::execution::service::CommonOperatorEngine;
use crate::application::file_dag::program::prepared_execution::types::{
    PreparedExecutionProgram, ProgramParts,
};
use crate::outbound::operators::interface::OperatorsPort;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::prepared_request::types::{Compiled, ObservationIdentity};
use ryo_execution_store::application::file_dag::derived::owner::service::DerivedAccess;
use ryo_values::domain::hosted::types::HostRelation;
use ryo_values::domain::rule_requests::types::RuleSpec;
use ryo_values::domain::stored_facts::types::Digest;
use std::sync::Arc;

/// The program of a request prepared over the engine's operators and the derived access: the access binds the
/// operators' derived views and opens every scope's relation owner, and the program a caller holds lends neither.
///
/// # Errors
///
/// The refusal of preparing the request's program: rules and digests that do not pair, an operator the domain cannot
/// prepare or whose derived views cannot be bound, and an incomplete named observation request.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::operating::errors::OperatingError;
/// use ryo_execution_domain::domain::prepared_request::types::{Compiled, ObservationIdentity};
/// use ryo_execution_runtime::application::file_dag::derived::service::prepare;
/// use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
/// use ryo_execution_runtime::outbound::digest::interface::DigestPort;
/// use ryo_execution_runtime::outbound::operators::interface::OperatorsPort;
/// use ryo_execution_store::application::file_dag::derived::owner::service::DerivedAccess;
/// use std::sync::Arc;
/// fn prepared(
///     compiled: Compiled,
///     operators: Arc<dyn OperatorsPort>,
///     derived: DerivedAccess,
///     digest: Arc<dyn DigestPort>,
/// ) -> Result<PreparedExecutionProgram, OperatingError> {
///     prepare(compiled, &[], &[], &[], ObservationIdentity::Independent, operators, derived, digest)
/// }
/// ```
pub fn prepare(
    compiled: Compiled,
    rules: &[RuleSpec],
    digests: &[Digest],
    hosted: &[HostRelation],
    identity: ObservationIdentity,
    operators: Arc<dyn OperatorsPort>,
    derived: DerivedAccess,
    digest: Arc<dyn crate::outbound::digest::interface::DigestPort>,
) -> Result<PreparedExecutionProgram, OperatingError> {
    let derived = ProgramDerived::from(derived);
    // Every file runs through the file pass, over the compiled plans the compilation shares: its program is planned here, once,
    // with each rule's stage, so a rule no stage serves yet is refused where it would run.
    let pass = crate::application::file_dag::file_pass::service::pass_program(
        &crate::application::file_dag::file_pass::types::PassSource {
            plans: compiled.plans.clone(),
            bindings: compiled.bindings.clone(),
            operators: Arc::clone(&operators),
        },
    )?;
    let inner = crate::application::file_dag::program::preparation::service::prepare(
        compiled,
        rules,
        digests,
        hosted,
        derived.clone(),
        identity,
    )?;
    Ok(PreparedExecutionProgram {
        parts: ProgramParts {
            inner,
            kernels: Arc::new(CommonOperatorEngine::new(operators)),
            derived,
            digest,
            pass: Arc::new(pass),
        },
    })
}
