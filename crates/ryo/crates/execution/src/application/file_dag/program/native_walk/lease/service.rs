//! Private call data and its already-admitted semantic dispatch.
use crate::application::file_dag::program::{
    preparation::types::PreparedProgram, runtime::service::KernelPermit,
};
use crate::domain::{
    operating::{errors::OperatingError, types::OperatorContext},
    step_classes::types::ClassPlace,
    shared_walk::types::{WalkWork, native_batch::value::NativeBatch},
};
/// Only the live engine permit can bind these borrowed, already-admitted rows.
pub(crate) struct PermittedWalk<'call, 'context, 'scope> {
    permit: &'call KernelPermit<'scope>,
    prepared: &'call PreparedProgram,
    source: crate::application::file_dag::program::admission::AdmittedWalkSource<'call>,
    context: &'call mut OperatorContext<'context>,
    ready: &'call [ClassPlace],
    work: &'call mut WalkWork,
}
impl<'call, 'context, 'scope> PermittedWalk<'call, 'context, 'scope> {
    pub(in crate::application::file_dag::program::native_walk) fn new(
        permit: &'call KernelPermit<'scope>,
        prepared: &'call PreparedProgram,
        source: crate::application::file_dag::program::admission::AdmittedWalkSource<'call>,
        context: &'call mut OperatorContext<'context>,
        ready: &'call [ClassPlace],
        work: &'call mut WalkWork,
    ) -> Self {
        Self {
            permit,
            prepared,
            source,
            context,
            ready,
            work,
        }
    }
}
impl PermittedWalk<'_, '_, '_> {
    pub(in crate::application::file_dag) fn dispatch(self) -> Result<NativeBatch, OperatingError> {
        self.permit.validate_program(self.prepared.program())?;
        self.permit.validate_walk_source(&self.source)?;
        let batch = crate::domain::shared_walk::service::evaluate(
            self.prepared.walk(),
            self.source.relation(),
            self.source.rows(),
            self.prepared.all_bindings(),
            self.context,
            self.ready,
            self.work,
        );
        let counts = batch.counts();
        tracing::trace!(target: crate::application::file_dag::program::constants::WALK_TARGET, event = crate::application::file_dag::program::constants::WALK_DISPATCH,
            relation = ?self.source.relation(), population = ?self.source.population(), generation = ?self.permit.diagnostic_generation(),
            phase = ?self.prepared.program().regions()[self.permit.region().0].phase(),
            classes = ?self.ready.iter().filter(|class| batch.class(**class).is_some()).map(|class| class.0).collect::<Vec<_>>(),
            passes = counts.passes.0, source_rows = counts.source_rows.0,
            decoded_rows = counts.decoded_rows.0, predicate_rows = counts.predicate_rows.0,
            predicate_tests = counts.predicate_tests.0, predicate_reuses = counts.predicate_reuses.0,
            result_slots = counts.result_slots.0);
        if let Some(reason) = batch.refusal() {
            tracing::trace!(target: crate::application::file_dag::program::constants::WALK_TARGET,
                event = crate::application::file_dag::program::constants::WALK_REFUSED,
                relation = ?self.source.relation(), population = ?self.source.population(),
                generation = ?self.permit.diagnostic_generation(), classes = ?self.ready.iter().map(|class| class.0).collect::<Vec<_>>(),
                reason = ?reason);
        }
        Ok(batch)
    }
}
