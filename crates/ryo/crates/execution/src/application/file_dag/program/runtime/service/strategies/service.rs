//! Whole-region strategies that publish without the ordinary kernel loop: blocked, ordered-existence and column regions.
use super::super::{InvocationOutcome, Poll, ProgramExecution};
use super::super::{KernelAdmission, KernelPermit};
use crate::application::file_dag::program::runtime::types::InvocationFailure;
use crate::application::file_dag::program::runtime::types::{ColumnCounters, StrategyOutcome};
use crate::domain::operating::errors::OperatingError;

impl ProgramExecution {
    pub(in crate::application::file_dag::program::runtime::service) fn blocked(
        &self,
        call: &mut Poll<'_, '_>,
    ) -> Result<Option<InvocationOutcome>, InvocationFailure> {
        let id = call.id;
        let region = call.region;
        let completion = call.completion;
        let inputs = &mut *call.inputs;
        region
            .instructions
            .first()
            .ok_or_else(|| InvocationFailure::Semantic(OperatingError::NotRun))?;
        let needs = region
            .needs
            .as_ref()
            .map_err(|error| InvocationFailure::Semantic(error.clone()))?;
        if completion.region_admission(needs)
            == crate::domain::completion::types::RegionAdmission::Blocked
        {
            let last = region
                .instructions
                .last()
                .ok_or_else(|| InvocationFailure::Semantic(OperatingError::NotRun))?;
            let input = inputs
                .publish_empty(
                    crate::application::file_dag::program::admission::ProducerRoot::Region(id),
                    &self.partition,
                    &[],
                    last.contract.output_kinds.clone(),
                )
                .map_err(InvocationFailure::Semantic)?;
            return Ok(Some(InvocationOutcome::Published(
                input.with_coverage(completion.blocked_coverage()),
            )));
        }
        Ok(None)
    }
    pub(in crate::application::file_dag::program::runtime::service) fn ordered(
        &self,
        call: &mut Poll<'_, '_>,
    ) -> Result<Option<InvocationOutcome>, InvocationFailure> {
        let id = call.id;
        let region = call.region;
        let completion = call.completion;
        let kernels = call.kernels;
        let inputs = &mut *call.inputs;
        let context = &mut *call.context;
        let progress = &mut *call.progress;
        let needs = region
            .needs
            .as_ref()
            .map_err(|error| InvocationFailure::Semantic(error.clone()))?;
        if let Some(shape) = region
            .ordered_existence
            .as_ref()
            .filter(|_| !progress.ordered_done)
        {
            progress
                .supply(
                    inputs,
                    crate::domain::executable_program::types::InputSource::Region(shape.source),
                    &self.partition,
                    context,
                )
                .map_err(InvocationFailure::Semantic)?;
            progress
                .check_schema(region)
                .map_err(InvocationFailure::Semantic)?;
            let source = progress
                .admit(
                    inputs,
                    crate::domain::executable_program::types::InputSource::Region(shape.source),
                    &self.partition,
                    context,
                )
                .map_err(InvocationFailure::Semantic)?;
            let permit = KernelPermit::ordered(
                KernelAdmission {
                    execution: self,
                    region: id,
                },
                &source,
            )
            .map_err(InvocationFailure::Semantic)?;
            let last = region
                .instructions
                .last()
                .ok_or_else(|| InvocationFailure::Semantic(OperatingError::NotRun))?;
            let mut rows = ryo_values::domain::rows::types::Table::default();
            ryo_values::domain::rows::service::reset(
                &mut rows,
                last.contract
                    .output_kinds
                    .as_ref()
                    .map_err(|error| InvocationFailure::Semantic(error.clone()))?,
            );
            let frames = cleanup
                .scratch
                .as_mut()
                .ok_or_else(|| InvocationFailure::Semantic(OperatingError::NotRun))?;
            let frame = frames
                .first_mut()
                .ok_or_else(|| InvocationFailure::Semantic(OperatingError::NotRun))?;
            let outcome = kernels.ordered(
                shape,
                &permit,
                &source,
                context,
                frame,
                &mut crate::domain::replay::types::Collect(&mut rows),
            )?;
            if matches!(outcome, StrategyOutcome::Executed) {
                completion.filter_rows(
                    region
                        .last_operation()
                        .map_err(InvocationFailure::Semantic)?,
                    needs,
                    &mut rows,
                );
                let coverage = source.coverage().map_err(InvocationFailure::Semantic)?;
                let publication = inputs
                    .publish(
                        crate::application::file_dag::program::admission::ProducerRoot::Region(id),
                        &self.partition,
                        &[source],
                        rows,
                    )
                    .map_err(InvocationFailure::Semantic)?
                    .with_coverage(coverage)
                    .with_coverage(completion.coverage(needs));

                return Ok(Some(InvocationOutcome::Published(publication)));
            }
        }
        progress.ordered_done = true;
        Ok(None)
    }
    pub(in crate::application::file_dag::program::runtime::service) fn column(
        &self,
        call: &mut Poll<'_, '_>,
    ) -> Result<Option<InvocationOutcome>, InvocationFailure> {
        let id = call.id;
        let region = call.region;
        let completion = call.completion;
        let kernels = call.kernels;
        let state = call.state;
        let inputs = &mut *call.inputs;
        let context = &mut *call.context;
        let progress = &mut *call.progress;
        let needs = region
            .needs
            .as_ref()
            .map_err(|error| InvocationFailure::Semantic(error.clone()))?;
        if let Some(shape) = region.column.as_ref().filter(|_| !progress.column_done) {
            use crate::domain::executable_program::types::InputSource;
            use crate::domain::executable_program::types::Instruction;
            progress
                .supply(
                    inputs,
                    InputSource::Region(shape.left()),
                    &self.partition,
                    context,
                )
                .map_err(InvocationFailure::Semantic)?;
            progress
                .supply(
                    inputs,
                    InputSource::Relation(shape.chain().right),
                    &self.partition,
                    context,
                )
                .map_err(InvocationFailure::Semantic)?;
            progress
                .check_schema(region)
                .map_err(InvocationFailure::Semantic)?;
            let left = progress
                .admit(
                    inputs,
                    InputSource::Region(shape.left()),
                    &self.partition,
                    context,
                )
                .map_err(InvocationFailure::Semantic)?;
            let right = progress
                .admit(
                    inputs,
                    InputSource::Relation(shape.chain().right),
                    &self.partition,
                    context,
                )
                .map_err(InvocationFailure::Semantic)?;
            let permit = KernelPermit::column(
                KernelAdmission {
                    execution: self,
                    region: id,
                },
                &left,
                &right,
            )
            .map_err(InvocationFailure::Semantic)?;
            let frames = cleanup
                .scratch
                .as_mut()
                .ok_or(InvocationFailure::Semantic(OperatingError::NotRun))?;
            let scratch = frames
                .first_mut()
                .ok_or(InvocationFailure::Semantic(OperatingError::NotRun))?;
            let counters = ColumnCounters::default();
            let mut rows = ryo_values::domain::rows::types::Table::default();
            let outcome = kernels.column(
                shape, &permit, &left, &right, context, scratch, &counters, &mut rows,
            );
            self.record_column(state, shape, &counters)
                .map_err(InvocationFailure::Semantic)?;
            if matches!(outcome?, StrategyOutcome::Executed) {
                completion.filter_rows(
                    region
                        .last_operation()
                        .map_err(InvocationFailure::Semantic)?,
                    needs,
                    &mut rows,
                );
                let publication = inputs
                    .publish(
                        crate::application::file_dag::program::admission::ProducerRoot::Region(id),
                        &self.partition,
                        &[left, right],
                        rows,
                    )
                    .map_err(InvocationFailure::Semantic)?
                    .with_coverage(completion.coverage(needs));
                return Ok(Some(InvocationOutcome::Published(publication)));
            }
        }
        progress.column_done = true;
        Ok(None)
    }
}
