//! Ordinary region evaluation: demand admission, preflight gates, readiness and the kernel continuation.
use super::super::admission::service::{empty_output_kinds, first_preflight};
use super::super::invocation::service::continuation;
use super::super::{InvocationOutcome, Poll, ProgramExecution};
use super::super::{KernelAdmission, KernelPermit, ReadySelection};
use crate::application::file_dag::program::admission::AdmittedInput;
use crate::application::file_dag::program::admission::Partition;
use crate::application::file_dag::program::runtime::types::InvocationFailure;
use crate::application::file_dag::program::runtime::types::KernelInput;
use crate::application::file_dag::program::runtime::types::{
    Emptiness, InstructionIndex, PollMode, StageProgress,
};
use crate::domain::executable_program::types::DemandAction;
use crate::domain::executable_program::types::InputId;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::types::PreflightOrder;
use std::sync::Arc;

impl ProgramExecution {
    pub(in crate::application::file_dag::program::runtime::service) fn admit_demand(
        &self,
        call: &mut Poll<'_, '_>,
        admitted: &mut Vec<AdmittedInput>,
    ) -> Result<Option<InvocationOutcome>, InvocationFailure> {
        let id = call.id;
        let region = call.region;
        let completion = call.completion;
        let inputs = &mut *call.inputs;
        let context = &mut *call.context;
        let progress = &mut *call.progress;
        let first = region
            .instructions
            .first()
            .ok_or_else(|| InvocationFailure::Semantic(OperatingError::NotRun))?;
        let needs = region
            .needs
            .as_ref()
            .map_err(|error| InvocationFailure::Semantic(error.clone()))?;
        let program = &first.demand;
        if first_preflight(region) == Some(PreflightOrder::SchemaFirst) {
            for action in program
                .actions()
                .iter()
                .skip(progress.availability_action.0)
            {
                match *action {
                    DemandAction::Admit(InputId(at)) => {
                        let source = *region.inputs.get(at).ok_or(InvocationFailure::Semantic(
                            OperatingError::MissingInput { position: at + 1 },
                        ))?;
                        progress
                            .supply(inputs, source, &self.partition, context)
                            .map_err(InvocationFailure::Semantic)?;
                    }
                }
                progress.availability_action.0 += 1;
            }
            progress
                .check_schema(region)
                .map_err(InvocationFailure::Semantic)?;
        }
        for action in program.actions().iter().skip(progress.action.0) {
            match *action {
                DemandAction::Admit(InputId(at)) => {
                    let source = *region.inputs.get(at).ok_or_else(|| {
                        InvocationFailure::Semantic(OperatingError::MissingInput {
                            position: at + 1,
                        })
                    })?;
                    admitted.push(
                        progress
                            .admit(inputs, source, &self.partition, context)
                            .map_err(InvocationFailure::Semantic)?,
                    );
                }
            }
            progress.action.0 += 1;
        }
        if progress.preflight_done == StageProgress::Pending {
            for gate in &first.contract.empty_results {
                let input = admitted.get(gate.input).ok_or(InvocationFailure::Semantic(
                    OperatingError::MissingInput {
                        position: gate.input + 1,
                    },
                ))?;
                if progress
                    .is_empty(
                        inputs,
                        *region
                            .inputs
                            .get(gate.input)
                            .ok_or(InvocationFailure::Semantic(OperatingError::MissingInput {
                                position: gate.input + 1,
                            }))?,
                        input,
                        gate.representation,
                    )
                    .map_err(InvocationFailure::Semantic)?
                    == Emptiness::Empty
                {
                    let publication = inputs
                        .publish_empty(
                            crate::application::file_dag::program::admission::ProducerRoot::Region(
                                id,
                            ),
                            &self.partition,
                            &admitted,
                            empty_output_kinds(region, gate),
                        )
                        .map_err(InvocationFailure::Semantic)?;
                    return Ok(Some(InvocationOutcome::Published(
                        publication.with_coverage(completion.coverage(needs)),
                    )));
                }
            }
        }
        progress
            .check_schema(region)
            .map_err(InvocationFailure::Semantic)?;
        progress.preflight_done = StageProgress::Done;
        Ok(None)
    }
    pub(in crate::application::file_dag::program::runtime::service) fn evaluate(
        &self,
        call: &mut Poll<'_, '_>,
        admitted: &[AdmittedInput],
    ) -> Result<InvocationOutcome, InvocationFailure> {
        let id = call.id;
        let region = call.region;
        let completion = call.completion;
        let kernels = call.kernels;
        let prepare_only = call.prepare_only;
        let inputs = &mut *call.inputs;
        let context = &mut *call.context;
        let progress = &mut *call.progress;
        let first = region
            .instructions
            .first()
            .ok_or_else(|| InvocationFailure::Semantic(OperatingError::NotRun))?;
        let needs = region
            .needs
            .as_ref()
            .map_err(|error| InvocationFailure::Semantic(error.clone()))?;
        let permit = KernelPermit::admitted(
            KernelAdmission {
                execution: self,
                region: id,
            },
            &admitted,
        )
        .map_err(InvocationFailure::Semantic)?;
        if progress.ready == StageProgress::Pending {
            // Observe the runtime obligation before the callback can omit it.
            // Schema and scope admission above established valid source geometry;
            // no PreparedWalk class or submission result selects this inventory.
            if tracing::enabled!(target: crate::application::file_dag::program::constants::WALK_TARGET, tracing::Level::TRACE)
                && region.phase == ryo_values::domain::plans::types::StepScope::PerFile
            {
                use ryo_values::domain::{
                    plan_parts::types::{Input, RepeatedRows},
                    plans::types::Operation,
                };
                let relation = match &first.operation {
                    Operation::Select {
                        from: Input::Relation(relation),
                        ..
                    }
                    | Operation::Project {
                        from: Input::Relation(relation),
                        distinct: RepeatedRows::Kept,
                        ..
                    } => Some(*relation),
                    _ => None,
                };
                if let (Partition::File(file), Some(relation)) = (&self.partition, relation) {
                    let population = admitted
                        .first()
                        .ok_or(OperatingError::MissingInput { position: 1 })
                        .and_then(|input| input.population(&self.owner));
                    // Log the complete proof result: an invalid proof cannot silently
                    // disappear from diagnostics or change execution error ordering.
                    tracing::trace!(target: crate::application::file_dag::program::constants::WALK_TARGET,
                        event = crate::application::file_dag::program::constants::WALK_READY,
                        logical = first.logical.0, region = id.0, relation = ?relation,
                        phase = ?region.phase, file = file.file().0,
                        generation = ?Arc::as_ptr(&self.generation), population = ?population);
                }
            }
            kernels
                .ready(&ReadySelection { permit: &permit }, &admitted, context)
                .map_err(InvocationFailure::Semantic)?;
            progress.ready = StageProgress::Done;
        }
        if prepare_only == PollMode::PrepareOnly {
            return Ok(InvocationOutcome::Ready);
        }
        let frames = call
            .frames
            .as_mut()
            .ok_or_else(|| InvocationFailure::Semantic(OperatingError::NotRun))?;
        let last = region
            .instructions
            .last()
            .ok_or_else(|| InvocationFailure::Semantic(OperatingError::NotRun))?;
        if region.native_positions.is_some() {
            if completion.row_retention()
                == crate::domain::completion::types::RowRetention::Unnecessary
            {
                let frame = frames
                    .first_mut()
                    .ok_or(InvocationFailure::Semantic(OperatingError::NotRun))?;
                if let Some(produced) = kernels.positions(&permit, context, frame)? {
                    let publication = inputs
                        .publish_positions(
                            crate::application::file_dag::program::admission::ProducerRoot::Region(
                                id,
                            ),
                            &self.partition,
                            &admitted,
                            produced,
                        )
                        .map_err(InvocationFailure::Semantic)?
                        .with_coverage(completion.coverage(needs));
                    return Ok(InvocationOutcome::Published(publication));
                }
            }
        }
        if let Some(shape) = &region.selection {
            let frame = frames
                .first_mut()
                .ok_or(InvocationFailure::Semantic(OperatingError::NotRun))?;
            if let Some(produced) = kernels.selected(shape, &permit, context, frame)? {
                let publication = inputs
                    .publish_selected(
                        crate::application::file_dag::program::admission::ProducerRoot::Region(id),
                        &self.partition,
                        &admitted,
                        produced,
                    )
                    .map_err(InvocationFailure::Semantic)?
                    .with_coverage(completion.coverage(needs));
                return Ok(InvocationOutcome::Published(publication));
            }
        }
        let mut rows = ryo_values::domain::rows::types::Table::default();
        let output_kinds = &last.contract.output_kinds;
        if let Ok(kinds) = output_kinds {
            ryo_values::domain::rows::service::reset(&mut rows, kinds);
        }
        continuation(
            &permit,
            region,
            InstructionIndex(0),
            KernelInput::Admitted(&admitted),
            kernels,
            context,
            frames,
            &mut crate::domain::replay::types::Collect(&mut rows),
        )?;
        output_kinds
            .as_ref()
            .map_err(|error| InvocationFailure::Semantic(error.clone()))?;
        completion.filter_rows(
            region
                .last_operation()
                .map_err(InvocationFailure::Semantic)?,
            needs,
            &mut rows,
        );
        let published = inputs
            .publish(
                crate::application::file_dag::program::admission::ProducerRoot::Region(id),
                &self.partition,
                &admitted,
                rows,
            )
            .map_err(InvocationFailure::Semantic)?;
        let published = if completion.observation_admission(needs)
            == crate::domain::completion::types::ObservationAdmission::Ready
        {
            published
        } else {
            published.with_coverage(completion.coverage(needs))
        };
        Ok(InvocationOutcome::Published(published))
    }
}
