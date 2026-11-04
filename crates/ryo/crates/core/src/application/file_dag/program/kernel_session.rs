//! Invocation-scoped resource acquisition followed by the mandatory semantic port.
use super::{
    admission::KernelPermit,
    runtime::{InvocationFailure, KernelInput, KernelInvocation, KernelScratch},
};
use crate::{
    application::file_dag::{
        executor::PrimitiveExecutor, invocation::InvocationResources, relations::RelationOwner,
        workspace::InvocationWorkspace,
    },
    domain::{
        executable_program::types::Instruction,
        operating::{
            errors::OperatingError,
            types::{Emit, InputPlace, OperatorContext},
        },
        operator::contract::{
            types::{EvaluationMode, InputShape, PreparedInputs},
            views::BorrowedRows,
        },
        table_model::{
            service::{count_of, read_row},
            types::Rows,
        },
    },
};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::rows::types::RowIndex;
use std::rc::Rc;

pub(super) struct KernelSession<'a, 'facts> {
    kernels: &'a dyn PrimitiveExecutor,
    owner: Rc<RelationOwner>,
    read: &'a dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
}
impl<'a, 'facts> KernelSession<'a, 'facts> {
    pub(super) fn new(
        kernels: &'a dyn PrimitiveExecutor,
        owner: Rc<RelationOwner>,
        read: &'a dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
    ) -> Self {
        Self {
            kernels,
            owner,
            read,
        }
    }
}
impl KernelInvocation for KernelSession<'_, '_> {
    fn evaluate(
        &self,
        instruction: &Instruction,
        permit: &KernelPermit<'_>,
        input: KernelInput<'_>,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
        output: &mut dyn Emit,
    ) -> Result<(), InvocationFailure> {
        let workspace = scratch
            .workspace
            .get_or_insert_with(|| InvocationWorkspace::new(Rc::clone(&self.owner)));
        let contract = instruction.contract.as_ref();
        match input {
            KernelInput::Continuation(row) => {
                let mut resources = InvocationResources::prepare(
                    contract,
                    &[],
                    context,
                    EvaluationMode::Streaming,
                    &[],
                    workspace,
                )
                .map_err(InvocationFailure::Semantic)?;
                crate::application::file_dag::execution::lease::evaluate(
                    self.kernels,
                    contract,
                    permit,
                    &instruction.operation,
                    PreparedInputs::Row {
                        input: InputPlace(0),
                        sides: &[],
                        row,
                    },
                    &mut resources,
                    workspace,
                    context,
                    output,
                )
                .map_err(InvocationFailure::Semantic)
            }
            KernelInput::Admitted(admitted, mode) => {
                let publications = admitted
                    .iter()
                    .map(|input| input.publication(&self.owner).cloned())
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(InvocationFailure::Semantic)?;
                let originals = publications
                    .iter()
                    .map(|publication| self.owner.resolve(publication.as_ref(), self.read))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(InvocationFailure::Semantic)?;
                let views: Vec<_> = originals.iter().map(BorrowedRows::from).collect();
                workspace.capture_admitted(publications.clone());
                let mut resources = InvocationResources::prepare(
                    contract, &views, context, mode, &originals, workspace,
                )
                .map_err(InvocationFailure::Semantic)?;
                let meaning = match mode {
                    EvaluationMode::Streaming => &contract.streaming,
                    EvaluationMode::Collected => &contract.collected,
                };
                match meaning.shape {
                    InputShape::Views => crate::application::file_dag::execution::lease::evaluate(
                        self.kernels,
                        contract,
                        permit,
                        &instruction.operation,
                        PreparedInputs::Views(&views),
                        &mut resources,
                        workspace,
                        context,
                        output,
                    )
                    .map_err(InvocationFailure::Semantic),
                    InputShape::RequestSource => {
                        let source = views.first().ok_or(InvocationFailure::Semantic(
                            OperatingError::MissingInput { position: 1 },
                        ))?;
                        crate::application::file_dag::execution::lease::evaluate(
                            self.kernels,
                            contract,
                            permit,
                            &instruction.operation,
                            PreparedInputs::RequestSource(source),
                            &mut resources,
                            workspace,
                            context,
                            output,
                        )
                        .map_err(InvocationFailure::Semantic)
                    }
                    InputShape::Row { primary } => {
                        let source = originals.get(primary.0).ok_or(
                            InvocationFailure::Semantic(OperatingError::MissingInput {
                                position: primary.0 + 1,
                            }),
                        )?;
                        let mut cells = Vec::new();
                        for at in 0..count_of(source).0 {
                            let row = read_row(source, RowIndex(at), &mut cells).ok_or(
                                InvocationFailure::Semantic(OperatingError::ShapeMismatch),
                            )?;
                            crate::application::file_dag::execution::lease::evaluate(
                                self.kernels,
                                contract,
                                permit,
                                &instruction.operation,
                                PreparedInputs::Row {
                                    input: InputPlace(primary.0),
                                    sides: &views,
                                    row,
                                },
                                &mut resources,
                                workspace,
                                context,
                                output,
                            )
                            .map_err(InvocationFailure::Semantic)?;
                        }
                        Ok(())
                    }
                }
            }
        }
    }
}
