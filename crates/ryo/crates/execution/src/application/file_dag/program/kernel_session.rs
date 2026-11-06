//! Invocation-scoped resource acquisition followed by the mandatory semantic port.
use crate::application::file_dag::program::runtime::service::InvocationFailure;
use crate::application::file_dag::program::runtime::service::KernelInvocation;
use crate::application::file_dag::program::runtime::service::KernelPermit;
use crate::application::file_dag::program::runtime::types::KernelInput;
use crate::application::file_dag::program::runtime::types::KernelScratch;
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
            types::{InputShape, PreparedInputs},
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
    ) -> Self {
        Self {
            kernels,
            prepared,
            owner,
            read,
            walk: super::native_walk::WalkPartition::Project,
        }
    }
}
impl KernelInvocation for KernelSession<'_, '_> {
    fn ready(
        &self,
        ready: &crate::application::file_dag::program::runtime::service::ReadySelection<'_, '_>,
        inputs: &[super::admission::AdmittedInput],
        _context: &mut OperatorContext<'_>,
    ) -> Result<bool, OperatingError> {
        self.walk
            .submit(self.prepared, ready, inputs, &self.owner, self.read)
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
        self.walk
            .validate(permit)
            .map_err(InvocationFailure::Semantic)?;
        let mut rebound = OperatorContext {
            phase: context.phase,
            semantic: context.semantic,
            prepared_matchers: context.prepared_matchers,
            prepared_kinds: context.prepared_kinds,
            bindings: self
                .prepared
                .bindings(permit.descriptor().producer)
                .map_err(InvocationFailure::Semantic)?,
            symbols: ryo_language::domain::symbols::service::query_reborrow(&mut context.symbols),
            source: context.source,
            paths: context.paths,
            arenas: context.arenas,
            memo: &mut *context.memo,
            scratch: &mut *context.scratch,
        };
        let context = &mut rebound;
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
            return Ok(
                crate::application::file_dag::program::runtime::types::StrategyOutcome::Declined,
            );
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
        Ok(crate::application::file_dag::program::runtime::types::StrategyOutcome::Executed)
    }
    fn positions(
        &self,
        permit: &KernelPermit<'_>,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
    ) -> Result<Option<crate::application::file_dag::relations::PublishedInput>, InvocationFailure>
    {
        self.walk
            .validate(permit)
            .map_err(InvocationFailure::Semantic)?;
        permit
            .validate_program(self.prepared.program())
            .map_err(InvocationFailure::Semantic)?;
        let instruction = permit.descriptor();
        let shape = permit.native_shape().map_err(InvocationFailure::Semantic)?;
        let region = permit.region();
        let mut rebound = OperatorContext {
            phase: context.phase,
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
        let order = self
            .prepared
            .native_order(region)
            .map_err(InvocationFailure::Semantic)?;
        let mut derived = self
            .owner
            .derived
            .finite_raw(Rc::clone(&self.owner), order.clone())
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
        if matches!(
            permit.operation(),
            ryo_values::domain::plans::types::Operation::Join { .. }
        ) {
            // Native step joins consume RawKeyOrder, not an ordinary Equality
            // or optional NativeLookup payload. Check the actual compiled requirement.
            if matches!(
                permit.operation(),
                ryo_values::domain::plans::types::Operation::Join {
                    right: ryo_values::domain::plan_parts::types::Input::Step(_),
                    ..
                }
            ) && order.is_none()
            {
                return Err(InvocationFailure::Semantic(OperatingError::WrongOperation));
            }
            if let Some(prepared) = &order {
                prepared
                    .validate()
                    .map_err(crate::domain::operator::contract::derived::portable_fault)
                    .map_err(InvocationFailure::Semantic)?;
            }
        }
        let empty_native_join = matches!(
            permit.operation(),
            ryo_values::domain::plans::types::Operation::Join { .. }
        ) && rows.first().is_some_and(|rows| count_of(rows).0 == 0);
        let shared = match rows.as_slice() {
            [source] => self
                .walk
                .acquire(
                    self.prepared,
                    permit,
                    source,
                    super::native_walk::WalkDelivery::Positions,
                    &mut rebound,
                    self.kernels,
                )
                .map_err(InvocationFailure::Semantic)?,
            _ => None,
        };
        let result = if let Some(shared) = shared {
            // The shared pass already owns this head's predicate work. Preserve
            // its actual tree coordinates instead of materializing rows and
            // making every downstream native operator decline.
            let [Rows::Tree(tree)] = rows.as_slice() else {
                return Ok(None);
            };
            if shape.width.0 != 1 || shape.columns.iter().any(|column| column.slot.0 != 0) {
                return Err(InvocationFailure::Semantic(OperatingError::ShapeMismatch));
            }
            let selected = shared.result();
            workspace.work.found(0, shared.visits().0);
            selected
                .completion
                .clone()
                .map_err(InvocationFailure::Semantic)?;
            let offset = u32::from(tree.form == crate::domain::table_cells::types::TreeForm::Edges);
            let end = ryo_language::domain::elements::service::real_count(tree.elements).0;
            let slots = selected
                .positions
                .0
                .iter()
                .map(|row| {
                    row.checked_add(offset)
                        .filter(|position| {
                            usize::try_from(*position).is_ok_and(|position| position < end)
                        })
                        .ok_or(OperatingError::ShapeMismatch)
                })
                .collect::<Result<Vec<_>, _>>();
            // No cached Ref crosses publication or any reentrant delivery.
            slots.map(|slots| {
                Some(crate::domain::positions::types::Positions {
                    width: crate::domain::lowering::types::RowWidth(1),
                    slots: crate::domain::positions::types::Slots(slots),
                })
            })
        } else if empty_native_join {
            crate::domain::native_positions::rows::source_tree(&rows).map(|tree| {
                tree.map(|_| crate::domain::positions::types::Positions {
                    width: shape.width,
                    slots: crate::domain::positions::types::Slots::default(),
                })
            })
        } else {
            crate::domain::native_positions::rows::evaluate(
                &semantics,
                &mut derived,
                permit.operation(),
                &rows,
                shape,
                &mut rebound,
                &mut scratch.positions,
                &workspace.work,
            )
        };
        let Some(positions) = failure
            .into_inner()
            .map_or(result, Err)
            .map_err(InvocationFailure::Semantic)?
        else {
            return Ok(None);
        };
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
        let publication = crate::application::file_dag::relations::positioned::publish(
            &self.owner,
            source,
            &admitted,
            positions,
            std::sync::Arc::clone(shape),
            std::sync::Arc::from(
                instruction
                    .contract
                    .output_kinds
                    .clone()
                    .map_err(InvocationFailure::Semantic)?,
            ),
        )
        .map_err(InvocationFailure::Semantic)?;
        #[cfg(test)]
        {
            workspace.observation.native_positions += 1;
        }
        Ok(Some(publication))
    }
    fn selected(
        &self,
        shape: &crate::domain::executable_program::selection::SelectionAlternative,
        permit: &KernelPermit<'_>,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
    ) -> Result<Option<crate::application::file_dag::relations::PublishedInput>, InvocationFailure>
    {
        self.walk
            .validate(permit)
            .map_err(InvocationFailure::Semantic)?;
        let mut rebound = OperatorContext {
            phase: context.phase,
            semantic: context.semantic,
            prepared_matchers: context.prepared_matchers,
            prepared_kinds: context.prepared_kinds,
            bindings: self
                .prepared
                .bindings(permit.descriptor().producer)
                .map_err(InvocationFailure::Semantic)?,
            symbols: ryo_language::domain::symbols::service::query_reborrow(&mut context.symbols),
            source: context.source,
            paths: context.paths,
            arenas: context.arenas,
            memo: &mut *context.memo,
            scratch: &mut *context.scratch,
        };
        let context = &mut rebound;
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
            &self.walk,
            self.kernels,
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
        counts: &crate::application::file_dag::program::runtime::types::ColumnCounters,
        rows: &mut ryo_values::domain::rows::types::Table,
    ) -> Result<
        crate::application::file_dag::program::runtime::types::StrategyOutcome,
        InvocationFailure,
    > {
        self.walk
            .validate(permit)
            .map_err(InvocationFailure::Semantic)?;
        let mut rebound = OperatorContext {
            phase: context.phase,
            semantic: context.semantic,
            prepared_matchers: context.prepared_matchers,
            prepared_kinds: context.prepared_kinds,
            bindings: self
                .prepared
                .bindings(permit.descriptor().producer)
                .map_err(InvocationFailure::Semantic)?,
            symbols: ryo_language::domain::symbols::service::query_reborrow(&mut context.symbols),
            source: context.source,
            paths: context.paths,
            arenas: context.arenas,
            memo: &mut *context.memo,
            scratch: &mut *context.scratch,
        };
        let context = &mut rebound;
        let chain = shape.chain();
        let Some(index_plan) = &chain.index else {
            return Ok(
                crate::application::file_dag::program::runtime::types::StrategyOutcome::Declined,
            );
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
            return Ok(
                crate::application::file_dag::program::runtime::types::StrategyOutcome::Declined,
            );
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
            return Ok(
                crate::application::file_dag::program::runtime::types::StrategyOutcome::Declined,
            );
        }
        crate::domain::derived_views::service::validate(
            ryo_derived::domain::contract::types::DerivedKind::Equality,
            index_plan,
        )
        .map_err(InvocationFailure::Semantic)?;
        if count_of(&left_rows).0 == 0 {
            ryo_values::domain::rows::service::reset(
                rows,
                &permit.output_kinds().map_err(InvocationFailure::Semantic)?,
            );
            return Ok(
                crate::application::file_dag::program::runtime::types::StrategyOutcome::Executed,
            );
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
            &permit.output_kinds().map_err(InvocationFailure::Semantic)?,
            &mut measured,
        );
        counts.join.found(0, measured.covered_rows);
        counts.join.found(1, measured.joined_hits);
        counts.filter.found(0, measured.joined_hits);
        counts.project.found(0, measured.filtered_rows);
        if let Some(error) = failure.get() {
            return Err(InvocationFailure::Semantic(error.clone()));
        }
        match result.map_err(InvocationFailure::Semantic)? {
            Some(_) => Ok(
                crate::application::file_dag::program::runtime::types::StrategyOutcome::Executed,
            ),
            None => Ok(
                crate::application::file_dag::program::runtime::types::StrategyOutcome::Declined,
            ),
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
        self.walk
            .validate(permit)
            .map_err(InvocationFailure::Semantic)?;
        let mut rebound = OperatorContext {
            phase: context.phase,
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
                KernelInput::Admitted(admitted) => {
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
                    if let [source] = originals.as_slice() {
                        if let Some(shared) = self
                            .walk
                            .acquire(
                                self.prepared,
                                permit,
                                source,
                                super::native_walk::WalkDelivery::Rows,
                                context,
                                self.kernels,
                            )
                            .map_err(InvocationFailure::Semantic)?
                        {
                            use crate::domain::replay::types::SelectionColumns;
                            use ryo_values::domain::{
                                plan_parts::types::RepeatedRows, plans::types::Operation,
                            };
                            let kept = match &instruction.operation {
                                Operation::Select { kept, .. } if kept.is_empty() => {
                                    SelectionColumns::All
                                }
                                Operation::Select { kept, .. } => SelectionColumns::Columns(kept),
                                Operation::Project {
                                    columns,
                                    distinct: RepeatedRows::Kept,
                                    ..
                                } => SelectionColumns::Columns(columns),
                                _ => {
                                    return Err(InvocationFailure::Semantic(
                                        OperatingError::WrongOperation,
                                    ));
                                }
                            };
                            workspace.work.found(0, shared.visits().0);
                            return crate::domain::replay::service::selected(
                                shared.result(),
                                source,
                                kept,
                                context,
                                output,
                            )
                            .map_err(InvocationFailure::Semantic);
                        }
                    }
                    let views: Vec<_> = originals.iter().map(BorrowedRows::from).collect();
                    if contract.semantics.preflight
                        == crate::domain::operator::contract::types::PreflightOrder::SchemaFirst
                    {
                        contract
                            .output_kinds
                            .as_ref()
                            .map_err(|error| InvocationFailure::Semantic(error.clone()))?;
                    }

                    workspace.capture_admitted(publications.clone());
                    InvocationResources::validate_row_metadata(contract, &views)
                        .map_err(InvocationFailure::Semantic)?;
                    let meaning = &contract.semantics;
                    let mut resources = if matches!(meaning.shape, InputShape::Row { .. })
                    && meaning.acquisition
                        == crate::domain::operator::contract::types::AcquisitionBoundary::FirstRow
                {
                    None
                } else {
                    Some(
                        InvocationResources::prepare(
                            contract, &views, context, &originals, workspace, &setup_semantics,
                        )
                        .map_err(InvocationFailure::Semantic)?,
                    )
                };
                    {
                        contract
                            .output_kinds
                            .as_ref()
                            .map_err(|error| InvocationFailure::Semantic(error.clone()))?;
                    }
                    if matches!(
                        permit.native_join_eligibility(),
                        super::native_join::Eligibility::Native
                    ) && originals.first().is_some_and(|left| count_of(left).0 != 0)
                        && matches!(originals.get(1), Some(Rows::Tree(_)))
                    {
                        if resources.is_none() {
                            resources = Some(
                                InvocationResources::prepare(
                                    contract,
                                    &views,
                                    context,
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
                    let meaning = &contract.semantics;
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
                            let primary_shift = u32::try_from(primary.0).map_err(|_| {
                                InvocationFailure::Semantic(OperatingError::WrongOperation)
                            })?;
                            let primary_bit = 1_u8.checked_shl(primary_shift).ok_or(
                                InvocationFailure::Semantic(OperatingError::WrongOperation),
                            )?;
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
