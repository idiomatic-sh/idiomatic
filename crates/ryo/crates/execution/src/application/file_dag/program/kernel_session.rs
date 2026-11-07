//! Invocation-scoped resource acquisition followed by the mandatory semantic port.
use crate::application::file_dag::program::runtime::service::KernelInvocation;
use crate::application::file_dag::program::runtime::service::KernelPermit;
use crate::application::file_dag::program::runtime::types::InvocationFailure;
use crate::application::file_dag::program::runtime::types::KernelInput;
use crate::application::file_dag::program::runtime::types::KernelScratch;
use crate::{
    application::file_dag::{executor::PrimitiveExecutor, relations::RelationOwner},
    domain::{
        executable_program::types::instruction::value::Instruction,
        operating::{
            errors::OperatingError,
            types::{Emit, OperatorContext},
        },
        table_model::types::Rows,
    },
};
use ryo_relations::domain::relations::types::Relation;
use std::rc::Rc;

mod evaluation;
mod positions;
mod strategies;

pub(super) struct KernelSession<'a, 'facts> {
    kernels: &'a dyn PrimitiveExecutor,
    prepared: &'a crate::application::file_dag::program::preparation::types::PreparedProgram,
    owner: Rc<RelationOwner>,
    read: &'a dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
    walk: super::native_walk::WalkPartition<'a>,
}
impl<'a, 'facts> KernelSession<'a, 'facts> {
    pub(super) fn file(
        kernels: &'a dyn PrimitiveExecutor,
        prepared: &'a crate::application::file_dag::program::preparation::types::PreparedProgram,
        owner: Rc<RelationOwner>,
        read: &'a dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
        walk: &'a super::native_walk::FileWalk,
    ) -> Self {
        Self {
            kernels,
            prepared,
            owner,
            read,
            walk: super::native_walk::WalkPartition::File(walk),
        }
    }
    pub(super) fn project(
        kernels: &'a dyn PrimitiveExecutor,
        prepared: &'a crate::application::file_dag::program::preparation::types::PreparedProgram,
        owner: Rc<RelationOwner>,
        read: &'a dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
        walk: &'a super::native_walk::project::service::ProjectWalk,
    ) -> Self {
        Self {
            kernels,
            prepared,
            owner,
            read,
            walk: super::native_walk::WalkPartition::Project(walk),
        }
    }
}
impl KernelInvocation for KernelSession<'_, '_> {
    fn ready(
        &self,
        ready: &crate::application::file_dag::program::runtime::service::ReadySelection<'_, '_>,
        inputs: &[super::admission::AdmittedInput],
        _context: &mut OperatorContext<'_>,
    ) -> Result<(), OperatingError> {
        self.walk
            .submit(self.prepared, ready, inputs, &self.owner, self.read)
            .map(|_| ())
    }
    fn ordered(
        &self,
        shape: &crate::domain::executable_program::types::OrderedExistence,
        permit: &KernelPermit<'_>,
        input: &super::admission::AdmittedInput,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
        output: &mut dyn Emit,
    ) -> Result<
        crate::application::file_dag::program::runtime::types::StrategyOutcome,
        InvocationFailure,
    > {
        self.ordered_session(shape, permit, input, context, scratch, output)
    }
    fn positions(
        &self,
        permit: &KernelPermit<'_>,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
    ) -> Result<
        crate::application::file_dag::program::runtime::types::Attempt<
            crate::application::file_dag::relations::handle::service::PublishedInput,
        >,
        InvocationFailure,
    > {
        self.positions_session(permit, context, scratch)
    }
    fn selected(
        &self,
        shape: &crate::domain::executable_program::types::SelectionAlternative,
        permit: &KernelPermit<'_>,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
    ) -> Result<
        crate::application::file_dag::program::runtime::types::Attempt<
            crate::application::file_dag::relations::handle::service::PublishedInput,
        >,
        InvocationFailure,
    > {
        self.selected_session(shape, permit, context, scratch)
    }
    fn column(
        &self,
        shape: &crate::domain::executable_program::types::ColumnAlternative,
        permit: &KernelPermit<'_>,
        left: &super::admission::AdmittedInput,
        right: &super::admission::AdmittedInput,
        context: &mut OperatorContext<'_>,
        _scratch: &mut KernelScratch,
        counts: &crate::application::file_dag::program::runtime::types::ColumnCounters,
        rows: &mut ryo_values::domain::rows::types::Table,
    ) -> Result<
        crate::application::file_dag::program::runtime::types::StrategyOutcome,
        InvocationFailure,
    > {
        self.column_session(shape, permit, left, right, context, _scratch, counts, rows)
    }
    fn evaluate(
        &self,
        instruction: &Instruction,
        permit: &KernelPermit<'_>,
        input: KernelInput<'_>,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
        output: &mut dyn Emit,
    ) -> Result<(), InvocationFailure> {
        self.evaluate_session(instruction, permit, input, context, scratch, output)
    }
}
