//! Existing physical algorithms are implementations selected by the common engine.
//! They borrow application-owned rule state; no primitive gains scheduler/cache access.
use super::execution::CommonOperatorEngine;
use crate::application::file_dag::executor::StepMaterialization;
use crate::application::file_dag::pipeline::{
    types::compact::value::CompactInput,
    types::{LoweredTree, Stepper},
};
use crate::domain::column_kernel::types::KernelStrategy;
use crate::domain::fragment::{service as fragment, types::FragmentRun};
use crate::domain::lowering::types::Counting;
use crate::domain::operating::{
    errors::OperatingError,
    types::{Emit, OperatorContext, Room},
};
use crate::domain::operator::{contract::types::OperatorContract, types::StepPlace};
use crate::domain::replay::{
    service::scan,
    types::{Collect, Pipe},
};
use crate::domain::table_model::{
    service::reset,
    types::{Rows, Table},
};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::StepId;

impl CommonOperatorEngine<'_> {
    pub(crate) fn stream_step<'a, 'f>(
        &self,
        contract: &OperatorContract,
        stepper: &mut Stepper<'a, 'f>,
        id: StepPlace,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError>
    where
        'f: 'a,
    {
        if let LoweredTree::Tree(tree) = stepper.lowered
            && (stepper
                .layout
                .lowering
                .shapes
                .get(id.0)
                .is_some_and(Option::is_some)
                || stepper.layout.lowering.counting.get(id.0) == Some(&Counting::Counts))
        {
            if contract.kind == ryo_values::domain::plans::types::OperationKind::Join {
                let mut output =
                    super::execution::output::State::take(&mut room.scratch, &mut room.memo);
                let result = output.run(emit, |emit| {
                    self.lowered_step(stepper, id, tree, context, emit)
                });
                output.restore(&mut room.scratch, &mut room.memo);
                return result;
            }
            return self.lowered_step(stepper, id, tree, context, emit);
        }
        if stepper.counting == Counting::DoesNotCount
            && let Some(rows) = CompactInput::new(stepper).selected(id, context)?
        {
            let mut buffer = stepper.spare.pop().unwrap_or_default();
            let result = scan(&rows, &mut buffer, context, emit);
            stepper.spare.push(buffer);
            return result;
        }
        self.stream(contract, stepper, context, room, emit)
    }
    fn lowered_step<'a, 'f>(
        &self,
        stepper: &mut Stepper<'a, 'f>,
        id: StepPlace,
        tree: crate::domain::table_cells::types::TreeRows<'f>,
        context: &mut OperatorContext<'_>,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError>
    where
        'f: 'a,
    {
        let semantics = super::execution::PrimitiveSemanticsBridge {
            port: self.operators,
        };
        let mut run = FragmentRun {
            plan: stepper.plan,
            lowering: &stepper.layout.lowering,
            tree,
            room: &mut *stepper.made,
            classes: stepper.classes,
        };
        let step = StepId(u32::try_from(id.0).map_err(|_| OperatingError::NotRun)?);
        if stepper.counting == Counting::DoesNotCount {
            return if fragment::counts(&run, step) == Counting::Counts {
                fragment::emit_counts(&semantics, &mut run, step, context, emit)
            } else {
                fragment::make(&semantics, &mut run, step, context)?;
                fragment::emit(&mut run, step, context, emit)
            };
        }
        let mut made = 0;
        let mut counting = Pipe(|each: &mut OperatorContext<'_>, row: &[Value]| {
            made += 1;
            emit.row(each, row)
        });
        let result = if fragment::counts(&run, step) == Counting::Counts {
            fragment::emit_counts(&semantics, &mut run, step, context, &mut counting)
        } else {
            fragment::make(&semantics, &mut run, step, context)
                .and_then(|()| fragment::emit(&mut run, step, context, &mut counting))
        };
        crate::application::file_dag::pipeline::service::feed(
            stepper,
            id,
            crate::domain::operating::types::InputPlace(0),
            crate::domain::table_model::types::RowCount(made),
        );
        result
    }
    pub(crate) fn keep_step<'a, 'f>(
        &self,
        contract: &OperatorContract,
        stepper: &mut Stepper<'a, 'f>,
        id: StepPlace,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        target: &mut Table,
    ) -> Result<StepMaterialization, OperatingError>
    where
        'f: 'a,
    {
        if stepper.counting == Counting::DoesNotCount
            && let Some(Rows::Selected(rows)) = CompactInput::new(stepper).selected(id, context)?
        {
            return Ok(StepMaterialization::Selected(rows.selection));
        }
        if stepper.kernel_strategy == KernelStrategy::Column
            && stepper.counting == Counting::DoesNotCount
            && let Some(chain) = stepper
                .kernel
                .and_then(|program| program.0.get(id.0))
                .and_then(Option::as_ref)
            && column::execute(
                &super::execution::PrimitiveSemanticsBridge {
                    port: self.operators,
                },
                chain,
                stepper.read,
                stepper.selected,
                stepper.partial,
                stepper.relation_owner.as_deref(),
                stepper.dag_shared,
                context,
                target,
                stepper
                    .layout
                    .kinds
                    .get(id.0)
                    .ok_or(OperatingError::NotRun)?,
                stepper.kernel_counts,
            )?
        {
            return Ok(StepMaterialization::Materialized);
        }
        reset(
            target,
            stepper
                .layout
                .kinds
                .get(id.0)
                .ok_or(OperatingError::NotRun)?,
        );
        self.stream_step(contract, stepper, id, context, room, &mut Collect(target))?;
        Ok(StepMaterialization::Materialized)
    }
}

pub(crate) mod column;
pub(crate) mod join;
pub(crate) mod ready;
