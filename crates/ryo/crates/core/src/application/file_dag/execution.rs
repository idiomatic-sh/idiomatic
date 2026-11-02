//! The mandatory shared operator engine. Policies change demand/retention, never primitive code.
use super::{invocation::InvocationResources, workspace::InvocationWorkspace};
use crate::application::file_dag::feed::Feed;
use crate::domain::operating::{
    errors::OperatingError,
    types::{Emit, InputPlace, OperatorContext, Room},
};
use crate::domain::operator::contract::{types::*, views::BorrowedRows};
use crate::domain::replay::types::Collect;
use crate::domain::table_cells::types::RowIndex;
use crate::domain::table_model::{
    service::{push_row, reset},
    types::{RowCount, Rows, Table},
};
use ryo_language::domain::relations::types::Value;
mod lease;
mod transport;
#[derive(Debug)]
pub(crate) struct CommonOperatorEngine;
static EMPTY: Table = Table {
    kinds: Vec::new(),
    cells: Vec::new(),
    count: RowCount(0),
};

/// Ragged physical providers historically expose only readable rows. They are
/// not complete typed populations: normalize once before the validated boundary.
/// Explicit computed row counts remain strict and never take this fallback.
fn selected_schema(input: &Rows<'_>) -> Result<(), OperatingError> {
    if let Rows::Selected(view) = input {
        if view.selection.columns.len() != view.selection.kinds.len()
            || !view
                .selection
                .columns
                .iter()
                .zip(&view.selection.kinds)
                .all(|(column, kind)| view.source.kinds.get(column.0 as usize) == Some(kind))
        {
            return Err(OperatingError::ShapeMismatch);
        }
    }
    Ok(())
}

fn physical_fallbacks(
    inputs: &[Rows<'_>],
    workspace: &InvocationWorkspace,
    offset: usize,
) -> Result<Vec<Option<std::sync::Arc<Table>>>, OperatingError> {
    inputs
        .iter()
        .enumerate().map(|(place,input)| {
            selected_schema(input)?;
            if matches!(input,Rows::Stored(_)|Rows::Selected(_)){if let Some(normalized)=workspace.normalization(place+offset,input.clone()){return normalized}}
            let physical = match input {
                Rows::Stored(view) => Some(*view),
                Rows::Selected(view) => Some(view.source),
                _ => None,
            };
            let missing_selected_row = matches!(input, Rows::Selected(view) if view.selection.rows.iter().any(|row| {
                view.selection.columns.iter().any(|column| view.source.columns.get(column.0 as usize).is_none_or(|values| *row as usize >= values.0.len()))
            }));
            if missing_selected_row || physical.is_some_and(|view| !crate::domain::table_model::service::rectangular(view))
            {
                let mut table = Table::default();
                crate::domain::table_model::service::materialize(
                    input,
                    &mut table,
                    &mut Vec::new(),
                );
                Ok(Some(std::sync::Arc::new(table)))
            } else {
                Ok(None)
            }
        })
        .collect()
}

impl CommonOperatorEngine {
    pub(crate) fn collect(
        &self,
        contract: &OperatorContract,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError> {
        #[cfg(test)]
        {
            context.scratch.observation = Default::default();
        }
        let mut workspace = InvocationWorkspace::default();
        workspace.predicates = std::mem::take(&mut context.scratch.select_scratch);
        workspace.predicate_memo = std::mem::take(context.memo);
        let result = self.collect_scoped(contract, inputs, context, out, &mut workspace);
        context.scratch.select_scratch = std::mem::take(&mut workspace.predicates);
        *context.memo = std::mem::take(&mut workspace.predicate_memo);
        #[cfg(test)]
        {
            context.scratch.observation.local_resources_released = true;
        }
        result
    }
    fn collect_scoped(
        &self,
        contract: &OperatorContract,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
        workspace: &mut InvocationWorkspace,
    ) -> Result<(), OperatingError> {
        let provider_inputs = inputs;
        let normalized = physical_fallbacks(inputs, workspace, 0)?;
        let originals = inputs
            .iter()
            .zip(&normalized)
            .map(|(input, fallback)| {
                fallback
                    .as_ref()
                    .map_or_else(|| input.clone(), |table| Rows::Computed(table.as_ref()))
            })
            .collect::<Vec<_>>();
        let originals = originals.as_slice();
        let inputs = originals.iter().map(BorrowedRows::from).collect::<Vec<_>>();
        let mode = contract.collected;
        let required = contract
            .inputs
            .used
            .0
            .max(usize::from(contract.external_source));
        if inputs.len() < required {
            return Err(OperatingError::MissingInput {
                position: inputs.len() + 1,
            });
        }
        for gate in &contract.collected_empty {
            let source = inputs.get(gate.input).ok_or(OperatingError::MissingInput {
                position: gate.input + 1,
            })?;
            if source.count() == 0
                && (gate.representation == EmptyRepresentation::Any
                    || matches!(source, BorrowedRows::Computed(_)))
            {
                reset(out, gate.kinds.as_ref().map_err(Clone::clone)?);
                return Ok(());
            }
        }
        if mode.preflight == PreflightOrder::SchemaFirst {
            contract.collected_kinds.as_ref().map_err(Clone::clone)?;
        }
        if mode.preflight == PreflightOrder::SchemaFirst {
            for (place, input) in inputs.iter().enumerate() {
                if matches!(input, BorrowedRows::Computed(_)) {
                    workspace.validate(place, *input)?;
                }
            }
            reset(
                out,
                contract.collected_kinds.as_ref().map_err(Clone::clone)?,
            );
            if let Some(result) = super::optimized::join::evaluate(
                &contract.resources,
                ryo_values::domain::plans::service::named(&contract.inputs),
                provider_inputs,
                context,
                &mut Collect(out),
            ) {
                return result;
            }
        }
        reset(&mut workspace.distinct, &contract.set_kinds);
        let mut resources = if mode.acquisition == AcquisitionBoundary::BeforeRows {
            Some(InvocationResources::prepare(
                contract,
                &inputs,
                context,
                mode.evaluation,
                originals,
                workspace,
            )?)
        } else {
            None
        };
        reset(
            out,
            contract.collected_kinds.as_ref().map_err(Clone::clone)?,
        );
        let result = self.run(
            contract,
            mode,
            &inputs,
            originals,
            &mut resources,
            workspace,
            context,
            &mut Collect(out),
        );
        #[cfg(test)]
        {
            if let Some(resources) = resources.as_ref() {
                resources.observe(workspace);
            }
            *context.scratch.observation = workspace.measured();
        }
        result
    }
    pub(crate) fn stream(
        &self,
        contract: &OperatorContract,
        feed: &mut dyn Feed<'_>,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        transport::stream(self, contract, feed, context, room, emit)
    }
    fn run<'f>(
        &self,
        contract: &OperatorContract,
        mode: ModeContract,
        inputs: &[BorrowedRows<'f>],
        originals: &[Rows<'f>],
        resources: &mut Option<InvocationResources<'f>>,
        workspace: &mut InvocationWorkspace,
        context: &mut OperatorContext<'_>,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        for input in inputs {
            let _bulk =
                crate::domain::table_model::collections::representations::service::registered(
                    crate::domain::table_model::collections::representations::service::kind(*input),
                )
                .bulk(*input)?;
        }
        if matches!(mode.shape, InputShape::Row { .. })
            && !originals.is_empty()
            && resources
                .as_ref()
                .is_none_or(|resources| resources.cached_graph.is_none())
            && super::optimized::ready::evaluate(contract, originals, context, emit)?
        {
            return Ok(());
        }
        match mode.shape {
            InputShape::Row { .. } => {
                let mut row = Vec::new();
                for place in 0..contract.inputs.used.0 {
                    if mode.row_inputs & (1 << place) == 0 {
                        continue;
                    }
                    let source = inputs.get(place).ok_or(OperatingError::MissingInput {
                        position: place + 1,
                    })?;
                    workspace.validate(place, *source)?;
                    for at in 0..source.count() {
                        {
                            let each = source
                                .read(RowIndex(at), &mut row)
                                .ok_or(OperatingError::ShapeMismatch)?;
                            self.row(
                                contract,
                                mode,
                                InputPlace(place),
                                each,
                                inputs,
                                originals,
                                resources,
                                workspace,
                                context,
                                emit,
                            )?;
                        }
                    }
                }
                Ok(())
            }
            InputShape::Views => {
                if resources.is_none() {
                    *resources = Some(InvocationResources::prepare(
                        contract,
                        inputs,
                        context,
                        mode.evaluation,
                        originals,
                        workspace,
                    )?);
                }
                for (place, input) in inputs.iter().enumerate() {
                    workspace.validate(place, *input)?;
                }
                if resources
                    .as_ref()
                    .is_some_and(|resources| resources.partitioned_closure.is_some())
                {
                    loop {
                        let more = resources
                            .as_mut()
                            .and_then(|resources| resources.partitioned_closure.as_mut())
                            .ok_or(OperatingError::NotRun)?
                            .advance()?;
                        if !more {
                            break;
                        }
                        if let Some(parts) = resources
                            .as_ref()
                            .and_then(|resources| resources.partitioned_closure.as_ref())
                        {
                            tracing::trace!(target:"ryo::diagnostic::lifecycle_memory",phase="closure_partition",owned_live_cells=parts.working_cells(),owned_capacity_cells=parts.working_capacity(),metadata_retained_bytes=parts.ordering_bytes());
                        }
                        lease::evaluate(
                            contract,
                            PreparedInputs::Views(inputs),
                            resources.as_mut().ok_or(OperatingError::NotRun)?,
                            workspace,
                            context,
                            emit,
                        )?;
                    }
                    return Ok(());
                }
                lease::evaluate(
                    contract,
                    PreparedInputs::Views(inputs),
                    resources.as_mut().ok_or(OperatingError::NotRun)?,
                    workspace,
                    context,
                    emit,
                )
            }
            InputShape::RequestSource => {
                if resources.is_none() {
                    *resources = Some(InvocationResources::prepare(
                        contract,
                        inputs,
                        context,
                        mode.evaluation,
                        originals,
                        workspace,
                    )?);
                }
                let source = inputs
                    .first()
                    .copied()
                    .unwrap_or(BorrowedRows::Computed(&EMPTY));
                workspace.validate(0, source)?;
                lease::evaluate(
                    contract,
                    PreparedInputs::RequestSource(&source),
                    resources.as_mut().ok_or(OperatingError::NotRun)?,
                    workspace,
                    context,
                    emit,
                )
            }
        }
    }
    fn row<'f>(
        &self,
        contract: &OperatorContract,
        mode: ModeContract,
        place: InputPlace,
        row: &[Value],
        inputs: &[BorrowedRows<'f>],
        originals: &[Rows<'f>],
        resources: &mut Option<InvocationResources<'f>>,
        workspace: &mut InvocationWorkspace,
        context: &mut OperatorContext<'_>,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        if resources.is_none() {
            *resources = Some(InvocationResources::prepare(
                contract,
                inputs,
                context,
                mode.evaluation,
                originals,
                workspace,
            )?);
        }
        if contract.set_scope == SetScope::PrimaryRow {
            reset(&mut workspace.distinct, &contract.set_kinds);
            workspace.seen.clear();
        }
        lease::evaluate(
            contract,
            PreparedInputs::Row {
                input: place,
                row,
                sides: inputs,
            },
            resources.as_mut().ok_or(OperatingError::NotRun)?,
            workspace,
            context,
            emit,
        )
    }
}

impl crate::application::file_dag::executor::PrimitiveExecutor for CommonOperatorEngine {
    fn stream(
        &self,
        contract: &OperatorContract,
        feed: &mut dyn Feed<'_>,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        CommonOperatorEngine::stream(self, contract, feed, context, room, emit)
    }
    fn collect(
        &self,
        contract: &OperatorContract,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError> {
        CommonOperatorEngine::collect(self, contract, inputs, context, out)
    }
    fn stream_step<'a, 'f>(
        &self,
        contract: &OperatorContract,
        stepper: &mut crate::application::file_dag::pipeline::types::Stepper<'a, 'f>,
        id: crate::domain::operator::types::StepPlace,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError>
    where
        'f: 'a,
    {
        CommonOperatorEngine::stream_step(self, contract, stepper, id, context, room, emit)
    }
    fn keep_step<'a, 'f>(
        &self,
        contract: &OperatorContract,
        stepper: &mut crate::application::file_dag::pipeline::types::Stepper<'a, 'f>,
        id: crate::domain::operator::types::StepPlace,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        target: &mut Table,
    ) -> Result<crate::application::file_dag::executor::StepMaterialization, OperatingError>
    where
        'f: 'a,
    {
        CommonOperatorEngine::keep_step(self, contract, stepper, id, context, room, target)
    }
}

#[cfg(test)]
mod tests;

impl CommonOperatorEngine {
    pub(super) fn keep_representation(
        &self,
        contract: &OperatorContract,
        physical:&crate::domain::table_model::collections::representations::publication::PreparedPhysical,
        feed: &mut super::service::GraphInputs<'_, '_>,
        owner: &super::relations::RelationOwner,
        context: &mut OperatorContext<'_>,
    ) -> Result<std::sync::Arc<super::relations::RelationResult>, OperatingError> {
        if let Some(which) = physical.alias.filter(|_| owner.stable()) {
            let result = feed.publication(which, context)?;
            feed.inspect_publication(result.as_ref(), owner)?;
            return Ok(result);
        }
        if let Some(selector) = physical.selector.as_ref() {
            if let Some(produced) = feed.produce(selector, owner, context)? {
                return owner.publish_selection(produced);
            }
        }
        let mut table = Table::default();
        reset(
            &mut table,
            &contract.collected_kinds.as_ref().map_err(Clone::clone)?,
        );
        self.stream(
            contract,
            feed,
            context,
            &mut Room::default(),
            &mut Collect(&mut table),
        )?;
        owner.computed(table)
    }
}
