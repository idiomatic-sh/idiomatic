//! Native position evaluation of a kernel session.
use crate::application::file_dag::program::runtime::service::KernelPermit;
use crate::application::file_dag::program::runtime::types::Attempt;
use crate::application::file_dag::program::runtime::types::InvocationFailure;
use crate::application::file_dag::program::runtime::types::KernelScratch;
use crate::domain::executable_program::types::InputId;
use crate::{
    application::file_dag::workspace::InvocationWorkspace,
    domain::{
        operating::{errors::OperatingError, types::OperatorContext},
        operator::contract::
        table_model::{service::count_of, types::Rows},
    },
};
use crate::domain::table_model::types::representation_input::value::RepresentationInput as BorrowedRows;
use std::rc::Rc;

impl super::super::KernelSession<'_, '_> {
    pub(in crate::application::file_dag::program::kernel_session) fn positions_session(
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
            matcher: context.matcher,
            prepared_kinds: context.prepared_kinds,
            bindings: self
                .prepared
                .bindings(instruction.producer())
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
                permit.descriptor().contract().as_ref(),
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
            .derived()
            .finite_raw(Rc::clone(&self.owner), order.clone(), &permit)
            .map_err(InvocationFailure::Semantic)?;
        let mut sources = Vec::new();
        let mut rows = Vec::new();
        for at in 0..instruction.inputs().len() {
            let admitted = permit
                .bound_input(InputId(at))
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
                    .semantic
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
                    crate::application::file_dag::program::native_walk::WalkDelivery::Positions,
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
                return Ok(Attempt::Declined);
            };
            if shape.width.0 != 1 || shape.columns.iter().any(|column| column.slot.0 != 0) {
                return Err(InvocationFailure::Semantic(OperatingError::ShapeMismatch));
            }
            let selected = shared.result();
            workspace.work.found(ryo_values::domain::plan_parts::types::InputNumber(0), crate::domain::operator::contract::types::FoundRows(shared.visits().0));
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
            derived.evaluate_native(
                &semantics,
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
            return Ok(Attempt::Declined);
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
                    .contract()
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
        Ok(Attempt::Produced(publication))
    }
}
