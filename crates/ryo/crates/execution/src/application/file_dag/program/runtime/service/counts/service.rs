//! Counters the execution reports: fed rows, populations, logical counts and readiness.
use super::super::{ProgramExecution, RuntimeState, Slot, State};
use crate::application::file_dag::program::runtime::types::ColumnCounters;
use crate::domain::operating::errors::OperatingError;

impl ProgramExecution {
    pub(in crate::application::file_dag::program::runtime::service) fn record_column(
        &self,
        state: &RuntimeState,
        shape: &crate::domain::executable_program::types::ColumnAlternative,
        counts: &ColumnCounters,
    ) -> Result<(), OperatingError> {
        let mut slots = state.slots.borrow_mut();
        for (id, values) in [
            (shape.join(), counts.join.snapshot()),
            (shape.filter(), counts.filter.snapshot()),
            (shape.target(), counts.project.snapshot()),
        ] {
            let slot = Slot::touched(state, &mut slots, &self.program, id)?;
            let fed = slot.fed.first_mut().ok_or(OperatingError::NotRun)?;
            for (total, found) in fed.0.iter_mut().zip(values) {
                *total = total.saturating_add(found);
            }
        }
        Ok(())
    }
    /// Consume the generation once no invocation can remain active. Dropping
    /// its slots releases only engine-held storage; transferred leases own theirs.
    pub(in crate::application::file_dag::program) fn state_counts(
        &self,
    ) -> crate::domain::readiness::types::InvocationStateCounts {
        use crate::domain::readiness::types::{InvocationStateCount, InvocationStateCounts};
        let states = self.project_states.borrow();
        let mut found = InvocationStateCounts {
            populations: InvocationStateCount(states.len()),
            ..Default::default()
        };
        for state in std::iter::once(&self.file_state)
            .chain(states.iter().map(|project| project.state.as_ref()))
        {
            let slots = state.slots.borrow();
            found.slots.0 += slots.len();
            found.instruction_counters.0 +=
                slots.values().map(|slot| slot.fed.len()).sum::<usize>();
            found.dormant_obligations.0 += state.dormant.borrow().len();
        }
        found
    }
    pub(in crate::application::file_dag::program) fn logical_counts(
        &self,
        state: &RuntimeState,
        rule: crate::domain::coverage::types::RuleIndex,
    ) -> Result<crate::application::file_dag::program::protocol::types::LogicalCounts, OperatingError>
    {
        self.validate_state(state)?;
        let rule = self
            .program
            .rules()
            .get(rule.0)
            .ok_or(OperatingError::NotRun)?;
        let slots = state.slots.borrow();
        let mut fed = Vec::with_capacity(rule.original_steps.len());
        for origin in rule.original_steps.iter() {
            let origin = origin.as_ref().map_err(Clone::clone)?;
            let region_id = *self
                .program
                .logical_regions()
                .get(origin.logical.0)
                .ok_or(OperatingError::NotRun)?;
            let code = self
                .program
                .regions()
                .get(region_id.0)
                .ok_or(OperatingError::NotRun)?;
            let at = code
                .instructions
                .iter()
                .position(|instruction| instruction.logical == origin.logical)
                .ok_or(OperatingError::NotRun)?;
            let Some(slot) = slots.get(&region_id) else {
                fed.push(crate::domain::operator::types::RowsFed::default());
                continue;
            };
            if matches!(slot.state, State::Running | State::Suspended | State::Ready) {
                return Err(OperatingError::NotRun);
            }
            let total = *slot.fed.get(at).ok_or(OperatingError::NotRun)?;
            fed.push(total);
        }
        Ok(
            crate::application::file_dag::program::protocol::types::LogicalCounts {
                fed: fed.into_boxed_slice(),
            },
        )
    }
    pub(in crate::application::file_dag::program) fn readiness_counts(
        &self,
    ) -> crate::domain::readiness::types::ReadinessCounts {
        *self.readiness_counts.borrow()
    }
}
