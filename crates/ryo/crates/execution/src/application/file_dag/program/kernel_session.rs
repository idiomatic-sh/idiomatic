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
        let rows = input
            .rows(&self.owner, self.read)
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
        let workspace = scratch.workspace.get_or_insert_with(|| {
            InvocationWorkspace::for_contract(
                Rc::clone(&self.owner),
                permit.descriptor().contract.as_ref(),
            )
        });
        let failure = std::sync::OnceLock::new();
        let semantics = crate::application::file_dag::execution::PrimitiveSemanticsBridge {
            port: self.kernels,
            permit: permit.semantics(),
            failure: &failure,
            binding: crate::domain::executable_program::physical::PhysicalBinding {
                bindings: context.bindings,
                regex_identity: None,
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
    fn positions(
        &self,
        permit: &KernelPermit<'_>,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
    ) -> Result<Option<crate::application::file_dag::relations::PublishedInput>, InvocationFailure>
    {
        permit
            .validate_program(self.prepared.program())
            .map_err(InvocationFailure::Semantic)?;
        let instruction = permit.descriptor();
        let shape = permit.native_shape().map_err(InvocationFailure::Semantic)?;
        let region = permit.region();
        if self
            .walk
            .is_some_and(|walk| walk.class(self.prepared, instruction).is_some())
        {
            return Ok(None);
        }
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
        let workspace = scratch.workspace.get_or_insert_with(|| {
            InvocationWorkspace::for_contract(
                Rc::clone(&self.owner),
                permit.descriptor().contract.as_ref(),
            )
        });
        let failure = std::sync::OnceLock::new();
        let semantics = crate::application::file_dag::execution::PrimitiveSemanticsBridge {
            port: self.kernels,
            permit: permit
                .position_semantics(shape)
                .map_err(InvocationFailure::Semantic)?,
            binding: crate::domain::executable_program::physical::PhysicalBinding {
                bindings: context.bindings,
                regex_identity: None,
                native_join: false,
                source_len: rebound.source.0.len(),
            },
            failure: &failure,
        };
        let mut derived = self
            .owner
            .derived
            .finite_raw(
                Rc::clone(&self.owner),
                self.prepared
                    .native_order(region)
                    .map_err(InvocationFailure::Semantic)?,
            )
            .map_err(InvocationFailure::Semantic)?;
        let mut sources = Vec::new();
        let mut rows = Vec::new();
        for at in 0..instruction.inputs.len() {
            let admitted = permit
                .bound_input(at)
                .map_err(InvocationFailure::Semantic)?;
            let publication = admitted
                .publication(&self.owner)
                .map_err(InvocationFailure::Semantic)?;
            let actual = admitted
                .rows(&self.owner, self.read)
                .map_err(InvocationFailure::Semantic)?;
            self.owner
                .admit(publication.as_ref(), BorrowedRows::from(&actual))
                .map_err(InvocationFailure::Semantic)?;
            sources.push(publication.clone());
            rows.push(actual);
        }
        let result = crate::domain::native_positions::rows::evaluate(
            &semantics,
            &mut derived,
            permit.operation(),
            &rows,
            shape,
            &mut rebound,
            &mut scratch.positions,
            &workspace.work,
        );
        let Some(positions) = failure
            .into_inner()
            .map_or(result, Err)
            .map_err(InvocationFailure::Semantic)?
        else {
            return Ok(None);
        };
        #[cfg(test)]
        {
            workspace.observation.native_positions += 1;
        }
        let source = sources
            .first()
            .ok_or(InvocationFailure::Semantic(OperatingError::NotRun))?;
        let admitted = self
            .owner
            .admit(
                source.as_ref(),
                BorrowedRows::from(
                    rows.first()
                        .ok_or(InvocationFailure::Semantic(OperatingError::NotRun))?,
                ),
            )
            .map_err(InvocationFailure::Semantic)?;
        crate::application::file_dag::relations::positioned::publish(
            &self.owner,
            source,
            &admitted,
            positions,
            std::sync::Arc::clone(shape),
            std::sync::Arc::from(
                instruction
                    .contract
                    .collected_kinds
                    .clone()
                    .map_err(InvocationFailure::Semantic)?,
            ),
        )
        .map(Some)
        .map_err(InvocationFailure::Semantic)
    }
    fn selected(
        &self,
        shape: &crate::domain::executable_program::selection::SelectionAlternative,
        permit: &KernelPermit<'_>,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
    ) -> Result<Option<crate::application::file_dag::relations::PublishedInput>, InvocationFailure>
    {
        let input = permit
            .selection_input(shape)
            .map_err(InvocationFailure::Semantic)?;
        let rows = input
            .rows(&self.owner, self.read)
            .map_err(InvocationFailure::Semantic)?;
        let workspace = scratch.workspace.get_or_insert_with(|| {
            InvocationWorkspace::for_contract(
                Rc::clone(&self.owner),
                permit.descriptor().contract.as_ref(),
            )
        });
        super::selection::acquire(
            self.prepared,
            permit,
            shape,
            &rows,
            &self.owner,
            context,
            &workspace.work,
        )
        .map_err(InvocationFailure::Semantic)
    }
    fn column(
        &self,
        shape: &crate::domain::executable_program::column::ColumnAlternative,
        permit: &KernelPermit<'_>,
        left: &super::admission::AdmittedInput,
        right: &super::admission::AdmittedInput,
        context: &mut OperatorContext<'_>,
        _scratch: &mut KernelScratch,
        counts: &super::runtime::ColumnCounters,
        rows: &mut ryo_values::domain::rows::types::Table,
    ) -> Result<super::runtime::StrategyOutcome, InvocationFailure> {
        let chain = shape.chain();
        let Some(index_plan) = &chain.index else {
            return Ok(super::runtime::StrategyOutcome::Declined);
        };
        let left_publication = left
            .publication(&self.owner)
            .map_err(InvocationFailure::Semantic)?;
        let right_publication = right
            .publication(&self.owner)
            .map_err(InvocationFailure::Semantic)?;
        let left_rows = left
            .rows(&self.owner, self.read)
            .map_err(InvocationFailure::Semantic)?;
        let right_rows = right
            .rows(&self.owner, self.read)
            .map_err(InvocationFailure::Semantic)?;
        let Rows::Stored(right_source) = &right_rows else {
            return Ok(super::runtime::StrategyOutcome::Declined);
        };
        use crate::domain::table_model::collections::representations::types::PhysicalShape;
        if self
            .owner
            .inspect(left_publication.as_ref(), BorrowedRows::from(&left_rows))
            .map_err(InvocationFailure::Semantic)?
            != PhysicalShape::Complete
            || self
                .owner
                .inspect(right_publication.as_ref(), BorrowedRows::from(&right_rows))
                .map_err(InvocationFailure::Semantic)?
                != PhysicalShape::Complete
            || index_plan
                .schemas
                .first()
                .is_none_or(|schema| schema.as_slice() != right_source.kinds)
        {
            return Ok(super::runtime::StrategyOutcome::Declined);
        }
        let acquired = self
            .owner
            .stored_equality(index_plan, chain.right, *right_source)
            .map_err(InvocationFailure::Semantic)?;
        let ryo_derived::domain::contract::types::DerivedView::Equality(index) =
            acquired.view().map_err(InvocationFailure::Semantic)?
        else {
            return Err(InvocationFailure::Semantic(OperatingError::WrongOperation));
        };
        counts.join.found(1, count_of(&right_rows).0 as u64);
        let failure = std::sync::OnceLock::new();
        let semantics = crate::application::file_dag::execution::PrimitiveSemanticsBridge {
            port: self.kernels,
            permit: permit.semantics(),
            binding: crate::domain::executable_program::physical::PhysicalBinding {
                bindings: context.bindings,
                regex_identity: None,
                native_join: false,
                source_len: context.source.0.len(),
            },
            failure: &failure,
        };
        let mut measured = crate::domain::column_kernel::types::KernelCounts::default();
        let result = crate::domain::column_kernel::service::execute_with_index(
            &semantics,
            chain,
            &left_rows,
            &right_rows,
            *right_source,
            index,
            context.symbols.as_ref(),
            ryo_language::domain::symbols::types::ByteSlice(context.source.0),
            rows,
            &permit
                .collected_kinds()
                .map_err(InvocationFailure::Semantic)?,
            &mut measured,
        );
        counts.join.found(0, measured.covered_rows);
        counts.filter.found(0, measured.joined_hits);
        counts.project.found(0, measured.filtered_rows);
        if let Some(error) = failure.get() {
            return Err(InvocationFailure::Semantic(error.clone()));
        }
        match result.map_err(InvocationFailure::Semantic)? {
            Some(_) => Ok(super::runtime::StrategyOutcome::Executed),
            None => Ok(super::runtime::StrategyOutcome::Declined),
        }
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
        let mut invocation_workspace = scratch.workspace.take().unwrap_or_else(|| {
            InvocationWorkspace::for_contract(
                Rc::clone(&self.owner),
                permit.descriptor().contract.as_ref(),
            )
        });
        let workspace = &mut invocation_workspace;
        let contract = instruction.contract.as_ref();
        let setup_failure = std::sync::OnceLock::new();
        let setup_semantics = crate::application::file_dag::execution::PrimitiveSemanticsBridge {
            port: self.kernels,
            permit: permit.semantics(),
            failure: &setup_failure,
            binding: crate::domain::executable_program::physical::PhysicalBinding {
                bindings: context.bindings,
                regex_identity: None,
                native_join: false,
                source_len: context.source.0.len(),
            },
        };
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
                    let originals = admitted
                        .iter()
                        .map(|input| input.rows(&self.owner, self.read))
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(InvocationFailure::Semantic)?;
                    if mode == EvaluationMode::Collected {
                        if let (Some(walk), [Rows::Tree(tree)]) = (self.walk, originals.as_slice())
                        {
                            if let Some(class) = walk.class(self.prepared, instruction) {
                                walk.prepare(self.prepared, permit, *tree, context, self.kernels)
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
                            contract, &views, context, mode, &originals, workspace, &setup_semantics,
                        )
                        .map_err(InvocationFailure::Semantic)?,
                    )
                };
                    if mode == EvaluationMode::Collected
                        && matches!(
                            permit.native_join_eligibility(),
                            super::native_join::Eligibility::Native
                        )
                        && originals.first().is_some_and(|left| count_of(left).0 != 0)
                        && matches!(originals.get(1), Some(Rows::Tree(_)))
                    {
                        if resources.is_none() {
                            resources = Some(
                                InvocationResources::prepare(
                                    contract,
                                    &views,
                                    context,
                                    mode,
                                    &originals,
                                    workspace,
                                    &setup_semantics,
                                )
                                .map_err(InvocationFailure::Semantic)?,
                            );
                        }
                        if let Some(native) = resources
                            .as_ref()
                            .ok_or(InvocationFailure::Semantic(OperatingError::NotRun))?
                            .native_join_lease()
                            .map_err(InvocationFailure::Semantic)?
                        {
                            let delivery = std::sync::OnceLock::new();
                            let request = super::native_join::PermittedNativeJoin::new(
                                permit,
                                native,
                                &originals[0],
                                &self.owner,
                                context,
                                &workspace.work,
                                output,
                                &delivery,
                            )
                            .map_err(InvocationFailure::Semantic)?;
                            let result = self.kernels.execute(
                                crate::application::file_dag::executor::KernelCall::NativeJoin {
                                    request,
                                },
                            );
                            if let Some(error) = delivery.get() {
                                return Err(InvocationFailure::Delivery(error.clone()));
                            }
                            return match result {
                                Ok(
                                    crate::application::file_dag::executor::KernelAnswer::Complete,
                                ) => Ok(()),
                                Ok(_) => {
                                    Err(InvocationFailure::Semantic(OperatingError::WrongOperation))
                                }
                                Err(
                                    crate::application::file_dag::executor::KernelFault::Refused(
                                        error,
                                    ),
                                ) => Err(InvocationFailure::Semantic(error)),
                                Err(
                                    crate::application::file_dag::executor::KernelFault::Portable(
                                        error,
                                    ),
                                ) => Err(InvocationFailure::Semantic(
                                    crate::domain::operator::contract::types::portable_fault(error),
                                )),
                            };
                        }
                    }
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
                            // The primary input defines the row contract; the required mask
                            // declares every population that drives that same evaluator.
                            let row_inputs = meaning.row_inputs.0;
                            let primary_bit = u32::try_from(primary.0)
                                .ok()
                                .and_then(|at| 1_u8.checked_shl(at))
                                .ok_or(InvocationFailure::Semantic(
                                    OperatingError::WrongOperation,
                                ))?;
                            if row_inputs == 0 || row_inputs & primary_bit == 0 {
                                return Err(InvocationFailure::Semantic(
                                    OperatingError::WrongOperation,
                                ));
                            }
                            for input in originals.len()..u8::BITS as usize {
                                if row_inputs & (1_u8 << input) != 0 {
                                    return Err(InvocationFailure::Semantic(
                                        OperatingError::MissingInput {
                                            position: input + 1,
                                        },
                                    ));
                                }
                            }
                            let mut cells = Vec::new();
                            for (input, source) in originals.iter().enumerate() {
                                if input >= u8::BITS as usize || row_inputs & (1_u8 << input) == 0 {
                                    continue;
                                }
                                for at in 0..count_of(source).0 {
                                    let row = read_row(source, RowIndex(at), &mut cells).ok_or(
                                        InvocationFailure::Semantic(OperatingError::ShapeMismatch),
                                    )?;
                                    workspace.work.found(input, 1);
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
                                                contract,
                                                &views,
                                                context,
                                                mode,
                                                &originals,
                                                workspace,
                                                &setup_semantics,
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
                                            input: InputPlace(input),
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
                            }
                            Ok(())
                        }
                    }
                }
            }
        })();
        let result = match setup_failure.into_inner() {
            Some(error) => Err(InvocationFailure::Semantic(error)),
            None => result,
        };
        workspace.release_inputs();
        scratch.workspace = Some(invocation_workspace);
        result
    }
}
