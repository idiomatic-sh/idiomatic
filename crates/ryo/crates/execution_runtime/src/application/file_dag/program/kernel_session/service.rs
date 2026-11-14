//! The `kernel_session` application operation.

use crate::application::file_dag::executor::service::PrimitiveExecutor;
use crate::application::file_dag::program::runtime::service::KernelInvocation;
use crate::application::file_dag::program::runtime::service::KernelPermit;
use crate::application::file_dag::program::runtime::types::InvocationFailure;
use crate::application::file_dag::program::runtime::types::KernelInput;
use crate::application::file_dag::program::runtime::types::KernelScratch;
use ryo_execution_domain::domain::operator::types::{Emit, OperatorContext};
use ryo_execution_domain::domain::{
    executable_program::types::instruction::value::Instruction,
    operating::errors::OperatingError,
    table_model::types::Rows,
};
use ryo_execution_store::application::file_dag::relations::service::RelationOwner;
use ryo_relations::domain::relations::types::Relation;
use std::rc::Rc;

pub(crate) struct KernelSession<'a, 'facts> {
    pub(super) kernels: &'a dyn PrimitiveExecutor,
    pub(super) prepared:
        &'a crate::application::file_dag::program::preparation::types::PreparedProgram,
    pub(super) owner: Rc<RelationOwner>,
    pub(super) read: &'a dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
    pub(super) walk: super::super::native_walk::service::project::service::WalkPartition<'a>,
}

impl<'a, 'facts> KernelSession<'a, 'facts> {
    pub(crate) fn file(
        kernels: &'a dyn PrimitiveExecutor,
        prepared: &'a crate::application::file_dag::program::preparation::types::PreparedProgram,
        owner: Rc<RelationOwner>,
        read: &'a dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
        walk: &'a super::super::native_walk::service::FileWalk,
    ) -> Self {
        Self {
            kernels,
            prepared,
            owner,
            read,
            walk: super::super::native_walk::service::project::service::WalkPartition::File(walk),
        }
    }
    pub(crate) fn project(
        kernels: &'a dyn PrimitiveExecutor,
        prepared: &'a crate::application::file_dag::program::preparation::types::PreparedProgram,
        owner: Rc<RelationOwner>,
        read: &'a dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
        walk: &'a super::super::native_walk::service::project::service::ProjectWalk,
    ) -> Self {
        Self {
            kernels,
            prepared,
            owner,
            read,
            walk: super::super::native_walk::service::project::service::WalkPartition::Project(
                walk,
            ),
        }
    }
}

impl KernelInvocation for KernelSession<'_, '_> {
    fn ready(
        &self,
        ready: &crate::application::file_dag::program::runtime::service::ReadySelection<'_, '_>,
        inputs: &[super::super::admission::service::AdmittedInput],
        _context: &mut OperatorContext<'_>,
    ) -> Result<super::super::runtime::types::Submission, OperatingError> {
        self.walk
            .submit(self.prepared, ready, inputs, &self.owner, self.read)
    }
    fn ordered(
        &self,
        shape: &ryo_execution_domain::domain::executable_program::types::OrderedExistence,
        permit: &KernelPermit<'_>,
        input: &super::super::admission::service::AdmittedInput,
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
