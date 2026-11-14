//! The `derived` application operation: a request's program prepared over the derived access, and the file preparation
//! of the derived owner.
use super::program::service::ProgramDerived;
use crate::application::file_dag::execution::service::CommonOperatorEngine;
use crate::application::file_dag::program::prepared_execution::types::{
    PreparedExecutionProgram, ProgramParts,
};
use crate::outbound::operators::interface::OperatorsPort;
use ryo_execution_domain::domain::coverage::types::{Partial, RuleIndex};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::prepared_request::types::{Compiled, ObservationIdentity};
use ryo_execution_store::application::file_dag::derived::owner::service::DerivedAccess;
use ryo_execution_store::application::file_dag::derived::prepared::service::RequestDerived;
use ryo_execution_store::application::file_dag::relations::service::RelationOwner;
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
    // Branch only, until the switch: the differential runs every file through the file pass.
    #[cfg(feature = "fused-differential")]
    let fused = Some(Arc::new(crate::application::file_dag::file_pass::types::PassSource {
        plans: compiled.plans.clone(),
        bindings: compiled.bindings.clone(),
        operators: Arc::clone(&operators),
    }));
    #[cfg(not(feature = "fused-differential"))]
    let fused = None;
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
            fused,
        },
    })
}


/// Prepares the file owner's derived views for the rules the file demands, over the request's prepared derived needs.
pub(crate) fn prepare_file(
    owner: &RelationOwner,
    prepared: &RequestDerived,
    demanded: &[RuleIndex],
    partial: &[Partial],
) -> Result<(), OperatingError> {
    owner
        .derived()
        .prepare_program_file(prepared, demanded, partial)
}
