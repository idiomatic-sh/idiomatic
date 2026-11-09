//! Invocation-scoped resource acquisition followed by the mandatory semantic port.
use crate::application::file_dag::program::runtime::service::KernelInvocation;
use crate::application::file_dag::program::runtime::service::KernelPermit;
use crate::application::file_dag::program::runtime::types::InvocationFailure;
use crate::application::file_dag::program::runtime::types::KernelInput;
use crate::application::file_dag::program::runtime::types::KernelScratch;
use crate::application::file_dag::executor::PrimitiveExecutor;
use ryo_execution_domain::domain::{executable_program::types::instruction::value::Instruction, operating::errors::OperatingError, table_model::types::Rows};
use ryo_execution_domain::domain::operator::types::{Emit, OperatorContext};
use ryo_execution_store::application::file_dag::relations::RelationOwner;
use ryo_relations::domain::relations::types::Relation;
use std::rc::Rc;

mod evaluation;
mod strategies;

pub(super) struct KernelSession<'a, 'facts> {
    kernels: &'a dyn PrimitiveExecutor,
    prepared: &'a crate::application::file_dag::program::preparation::types::PreparedProgram,
    owner: Rc<RelationOwner>,
    read: &'a dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
    walk: super::native_walk::project::service::WalkPartition<'a>,
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
            walk: super::native_walk::project::service::WalkPartition::File(walk),
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
            walk: super::native_walk::project::service::WalkPartition::Project(walk),
        }
    }
}
impl KernelInvocation for KernelSession<'_, '_> {
    fn ready(
        &self,
        ready: &crate::application::file_dag::program::runtime::service::ReadySelection<'_, '_>,
        inputs: &[super::admission::AdmittedInput],
        _context: &mut OperatorContext<'_>,
    ) -> Result<super::runtime::types::Submission, OperatingError> {
        self.walk
            .submit(self.prepared, ready, inputs, &self.owner, self.read)
    }
    fn ordered(
        &self,
        shape: &ryo_execution_domain::domain::executable_program::types::OrderedExistence,
        permit: &KernelPermit<'_>,
        input: &super::admission::AdmittedInput,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
        output: &mut dyn Emit,
    ) -> Result<(), InvocationFailure> {
        self.ordered_session(shape, permit, input, context, scratch, output)
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
