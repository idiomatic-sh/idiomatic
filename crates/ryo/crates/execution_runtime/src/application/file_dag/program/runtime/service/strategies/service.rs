//! Whole-region strategies that publish without the ordinary kernel loop: blocked, absent and ordered-existence regions.
use super::super::{InvocationOutcome, Poll, ProgramExecution};
use super::super::{KernelAdmission, KernelPermit};
use crate::application::file_dag::program::runtime::types::InvocationFailure;
use crate::application::file_dag::program::runtime::types::StageProgress;
use ryo_execution_domain::domain::operating::errors::OperatingError;

impl ProgramExecution {
    pub(crate) fn blocked(
        &self,
        call: &mut Poll<'_, '_>,
    ) -> Result<Option<InvocationOutcome>, InvocationFailure> {
        let id = call.id;
        let region = call.region;
        let completion = call.completion;
        let inputs = &mut *call.inputs;
        region
            .instructions()
            .first()
            .ok_or_else(|| InvocationFailure::Semantic(OperatingError::NotRun))?;
        let needs = region
            .needs()
            .as_ref()
            .map_err(|error| InvocationFailure::Semantic(error.clone()))?;
        if completion.region_admission(needs)
            == ryo_execution_domain::domain::completion::types::RegionAdmission::Blocked
        {
            let last = region
                .instructions()
                .last()
                .ok_or_else(|| InvocationFailure::Semantic(OperatingError::NotRun))?;
            let input = inputs
                .publish(
                    crate::application::file_dag::program::admission::types::ProducerRoot::Region(id),
                    &self.partition,
                    &[],
                    crate::application::file_dag::program::admission::service::Representation::empty(
                        last.contract().output_kinds.clone(),
                    )
                    .map_err(InvocationFailure::Semantic)?,
                )
                .map_err(InvocationFailure::Semantic)?;
            return Ok(Some(InvocationOutcome::Published(
                input.with_coverage(completion.blocked_coverage()),
            )));
        }
        Ok(None)
    }
    /// A region whose skip formula fails on the file has no row and no row error there: it publishes the empty table,
    /// complete, without demanding an input. Blocked regions publish first; the presence gate is off on any file with a
    /// partial read, so the two never meet.
    pub(crate) fn absent(
        &self,
        call: &mut Poll<'_, '_>,
    ) -> Result<Option<InvocationOutcome>, InvocationFailure> {
        let id = call.id;
        let region = call.region;
        let root = call.state.absent.borrow().presence(id);
        if root == ryo_execution_domain::domain::executable_program::presence::types::RegionPresence::Present && call.inputs.presence(id) == ryo_execution_domain::domain::executable_program::presence::types::RegionPresence::Present {
            return Ok(None);
        }
        let needs = region
            .needs()
            .as_ref()
            .map_err(|error| InvocationFailure::Semantic(error.clone()))?;
        let last = region
            .instructions()
            .last()
            .ok_or_else(|| InvocationFailure::Semantic(OperatingError::NotRun))?;
        let input = call
            .inputs
            .publish(
                crate::application::file_dag::program::admission::types::ProducerRoot::Region(id),
                &self.partition,
                &[],
                crate::application::file_dag::program::admission::service::Representation::empty(
                    last.contract().output_kinds.clone(),
                )
                .map_err(InvocationFailure::Semantic)?,
            )
            .map_err(InvocationFailure::Semantic)?;
        self.readiness_counts.borrow_mut().presence_absent.0 += 1;
        Ok(Some(InvocationOutcome::Published(
            input.with_coverage(call.completion.coverage(needs)),
        )))
    }
    pub(crate) fn ordered(
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
            .needs()
            .as_ref()
            .map_err(|error| InvocationFailure::Semantic(error.clone()))?;
        if let Some(shape) =
            ryo_execution_domain::domain::executable_program::service::ordered_existence(
                &region.route(),
            )
            .filter(|_| progress.ordered_done == StageProgress::Pending)
        {
            progress
                .supply(
                    inputs,
                    ryo_execution_domain::domain::executable_program::types::InputSource::Region(
                        shape.source,
                    ),
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
                    ryo_execution_domain::domain::executable_program::types::InputSource::Region(
                        shape.source,
                    ),
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
                .instructions()
                .last()
                .ok_or_else(|| InvocationFailure::Semantic(OperatingError::NotRun))?;
            let mut rows = ryo_values::domain::rows::types::Table::default();
            ryo_values::domain::rows::service::reset(
                &mut rows,
                last.contract()
                    .output_kinds
                    .as_ref()
                    .map_err(|error| InvocationFailure::Semantic(error.clone()))?,
            );
            let frames = super::super::invocation::service::frames(call.frames, region)?;
            let frame = frames
                .first_mut()
                .ok_or_else(|| InvocationFailure::Semantic(OperatingError::NotRun))?;
            kernels.ordered(
                shape,
                &permit,
                &source,
                context,
                frame,
                &mut ryo_execution_domain::domain::replay::types::Collect(&mut rows),
            )?;
            completion.filter_rows(
                region
                    .last_operation()
                    .map_err(InvocationFailure::Semantic)?,
                needs,
                &mut rows,
            );
            let coverage = source.coverage().clone();
            let publication = inputs
                .publish(
                    crate::application::file_dag::program::admission::types::ProducerRoot::Region(id),
                    &self.partition,
                    &[source],
                    crate::application::file_dag::program::admission::service::Representation::Computed(
                        rows,
                    ),
                )
                .map_err(InvocationFailure::Semantic)?
                .with_coverage(coverage)
                .with_coverage(completion.coverage(needs));

            return Ok(Some(InvocationOutcome::Published(publication)));
        }
        progress.ordered_done = StageProgress::Done;
        Ok(None)
    }
}
