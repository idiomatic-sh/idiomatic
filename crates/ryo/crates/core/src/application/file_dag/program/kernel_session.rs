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
    prepared: &'a super::facade::PreparedProgram,
    owner: Rc<RelationOwner>,
    read: &'a dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
    walk: Option<&'a super::native_walk::FileWalk>,
}
impl<'a, 'facts> KernelSession<'a, 'facts> {
    pub(super) fn new(
        kernels: &'a dyn PrimitiveExecutor,
        prepared: &'a super::facade::PreparedProgram,
        owner: Rc<RelationOwner>,
        read: &'a dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
        walk: Option<&'a super::native_walk::FileWalk>,
    ) -> Self {
        Self {
            kernels,
            prepared,
            owner,
            read,
            walk,
        }
    }
}
impl KernelInvocation for KernelSession<'_, '_> {
    fn ordered(
        &self,
        shape: &crate::domain::executable_program::types::OrderedExistence,
        permit: &KernelPermit<'_>,
        input: &super::admission::AdmittedInput,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
        output: &mut dyn Emit,
    ) -> Result<super::runtime::StrategyOutcome, InvocationFailure> {
        let publication = input
            .publication(&self.owner)
            .map_err(InvocationFailure::Semantic)?;
        let rows = self
            .owner
            .resolve(publication.as_ref(), self.read)
            .map_err(InvocationFailure::Semantic)?;
        let Some(number) = self
            .owner
            .admit_number(
                publication,
                shape.source,
                &shape.number,
                BorrowedRows::from(&rows),
            )
            .map_err(InvocationFailure::Semantic)?
        else {
            return Ok(super::runtime::StrategyOutcome::Declined);
        };
        let workspace = scratch
            .workspace
            .get_or_insert_with(|| InvocationWorkspace::new(Rc::clone(&self.owner)));
        let failure = std::sync::OnceLock::new();
        let semantics = crate::application::file_dag::execution::PrimitiveSemanticsBridge {
            port: self.kernels,
            permit,
            failure: &failure,
            binding: crate::domain::executable_program::physical::PhysicalBinding {
                native_join: false,
                source_len: context.source.0.len(),
            },
        };
        let result = crate::application::file_dag::ordered_existence::evaluate(
            &semantics,
            shape,
            &number,
            BorrowedRows::from(&rows),
            context,
            &mut scratch.ordered_row,
            &mut scratch.ordered,
            &workspace.work,
            output,
        );
        failure
            .into_inner()
            .map_or(result, Err)
            .map_err(InvocationFailure::Semantic)?;
        Ok(super::runtime::StrategyOutcome::Executed)
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
        let mut rebound = OperatorContext {
            semantic: context.semantic,
            prepared_matchers: context.prepared_matchers,
            prepared_kinds: context.prepared_kinds,
            bindings: self
                .prepared
                .bindings(instruction.producer)
                .map_err(InvocationFailure::Semantic)?,
            symbols: ryo_language::domain::symbols::service::query_reborrow(&mut context.symbols),
            source: context.source,
            paths: context.paths,
            arenas: context.arenas,
            memo: &mut *context.memo,
            scratch: &mut *context.scratch,
        };
        let context = &mut rebound;
        let mut invocation_workspace = scratch
            .workspace
            .take()
            .unwrap_or_else(|| InvocationWorkspace::new(Rc::clone(&self.owner)));
        let workspace = &mut invocation_workspace;
        let contract = instruction.contract.as_ref();
        let result = (|| {
            match input {
                KernelInput::Continuation(row) => {
                    workspace.work.found(0, 1);
                    let mut resources = InvocationResources::continuation(contract)
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
                    if mode == EvaluationMode::Collected {
                        if let (Some(walk), [Rows::Tree(tree)]) = (self.walk, originals.as_slice())
                        {
                            if let Some(class) = walk.class(self.prepared, instruction) {
                                walk.prepare(
                                    self.prepared,
                                    permit,
                                    &admitted[0],
                                    *tree,
                                    context,
                                    self.kernels,
                                )
                                .map_err(InvocationFailure::Semantic)?;
                                let batch = walk.batch();
                                let batch = batch
                                    .as_ref()
                                    .ok_or(InvocationFailure::Semantic(OperatingError::NotRun))?;
                                if let Some(result) = batch.class(class) {
                                    workspace.work.found(0, batch.visits(class).0);
                                    let positions = result.as_ref().map_err(|error| {
                                        InvocationFailure::Semantic(error.clone())
                                    })?;
                                    let tree_rows = Rows::Tree(*tree);
                                    let mut cells = Vec::new();
                                    for position in &positions.0 {
                                        let row = read_row(
                                            &tree_rows,
                                            RowIndex(*position as usize),
                                            &mut cells,
                                        )
                                        .ok_or(
                                            InvocationFailure::Semantic(OperatingError::NotRead),
                                        )?;
                                        output
                                            .row(context, row)
                                            .map_err(InvocationFailure::Delivery)?;
                                    }
                                    return Ok(());
                                }
                            }
                        }
                    }
                    let views: Vec<_> = originals.iter().map(BorrowedRows::from).collect();
                    workspace.capture_admitted(publications.clone());
                    let meaning = match mode {
                        EvaluationMode::Streaming => &contract.streaming,
                        EvaluationMode::Collected => &contract.collected,
                    };
                    let mut resources = if matches!(meaning.shape, InputShape::Row { .. })
                    && meaning.acquisition
                        == crate::domain::operator::contract::types::AcquisitionBoundary::FirstRow
                {
                    None
                } else {
                    Some(
                        InvocationResources::prepare(
                            contract, &views, context, mode, &originals, workspace,
                        )
                        .map_err(InvocationFailure::Semantic)?,
                    )
                };
                    let meaning = match mode {
                        EvaluationMode::Streaming => &contract.streaming,
                        EvaluationMode::Collected => &contract.collected,
                    };
                    match meaning.shape {
                        InputShape::Views => {
                            crate::application::file_dag::execution::lease::evaluate(
                                self.kernels,
                                contract,
                                permit,
                                &instruction.operation,
                                PreparedInputs::Views(&views),
                                resources
                                    .as_mut()
                                    .ok_or(InvocationFailure::Semantic(OperatingError::NotRun))?,
                                workspace,
                                context,
                                output,
                            )
                            .map_err(InvocationFailure::Semantic)
                        }
                        InputShape::RequestSource => {
                            let empty = ryo_values::domain::rows::types::Table::default();
                            let source = match &instruction.operation {
                                // Rows semantics consume the exact producer's parameter
                                // snapshot through the lease, not an external relation.
                                ryo_values::domain::plans::types::Operation::Rows { .. } => {
                                    BorrowedRows::Computed(&empty)
                                }
                                _ => views.first().copied().ok_or(InvocationFailure::Semantic(
                                    OperatingError::MissingInput { position: 1 },
                                ))?,
                            };
                            crate::application::file_dag::execution::lease::evaluate(
                                self.kernels,
                                contract,
                                permit,
                                &instruction.operation,
                                PreparedInputs::RequestSource(&source),
                                resources
                                    .as_mut()
                                    .ok_or(InvocationFailure::Semantic(OperatingError::NotRun))?,
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
                                workspace.work.found(primary.0, 1);
                                let texts = crate::domain::table_texts::types::Texts {
                                    symbols: context.symbols.as_ref(),
                                    source: context.source,
                                    paths: context.paths,
                                    arenas: context.arenas,
                                };
                                if !crate::domain::executable_program::kernel::row_admissible(
                                    contract, row, &texts,
                                )
                                .map_err(InvocationFailure::Semantic)?
                                {
                                    continue;
                                }
                                if resources.is_none() {
                                    resources = Some(
                                        InvocationResources::prepare(
                                            contract, &views, context, mode, &originals, workspace,
                                        )
                                        .map_err(InvocationFailure::Semantic)?,
                                    );
                                }
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
                                    resources.as_mut().ok_or(InvocationFailure::Semantic(
                                        OperatingError::NotRun,
                                    ))?,
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
        })();
        workspace.release_inputs();
        scratch.workspace = Some(invocation_workspace);
        result
    }
}
