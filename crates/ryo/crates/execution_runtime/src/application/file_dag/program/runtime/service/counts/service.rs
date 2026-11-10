//! Counters the execution reports: fed rows, populations, logical counts and readiness.
use super::super::{ProgramExecution, RuntimeState, State};
use crate::application::file_dag::program::runtime::types::FedRead;
use ryo_execution_domain::domain::operating::errors::OperatingError;

impl ProgramExecution {
    /// Consume the generation once no invocation can remain active. Dropping
    /// its slots releases only engine-held storage; transferred leases own theirs.
    #[cfg(any(test, feature = "fixtures"))]
    pub(crate) fn state_counts(
        &self,
    ) -> ryo_execution_domain::domain::readiness::types::InvocationStateCounts {
        use ryo_execution_domain::domain::readiness::types::{
            InvocationStateCount, InvocationStateCounts,
        };
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
    /// The rows each step of `rule` was fed, after every check of the rule's state: a stale or dead generation, a rule out of
    /// range, a step that was not placed or has not run. `CheckOnly` makes the same checks and collects nothing.
    pub(crate) fn logical_counts(
        &self,
        state: &RuntimeState,
        rule: ryo_execution_domain::domain::coverage::types::RuleIndex,
        read: FedRead,
    ) -> Result<ryo_execution_domain::domain::operator::types::LogicalCounts, OperatingError> {
        self.validate_state(state)?;
        let rule = self
            .program
            .rules()
            .get(rule.0)
            .ok_or_else(|| OperatingError::NotRun)?;
        let slots = state.slots.borrow();
        let mut fed = Vec::with_capacity(match read {
            FedRead::Collect => rule.original_steps.len(),
            FedRead::CheckOnly => 0,
        });
        for origin in rule.original_steps.iter() {
            let origin = origin.as_ref().map_err(Clone::clone)?;
            let site = origin.place.ok_or_else(|| OperatingError::NotRun)?;
            let total = match slots.get(&site.region) {
                None => ryo_execution_domain::domain::operator::types::RowsFed::default(),
                Some(slot) => {
                    if matches!(slot.state, State::Running | State::Suspended | State::Ready) {
                        return Err(OperatingError::NotRun);
                    }
                    *slot
                        .fed
                        .get(site.instruction.0)
                        .ok_or_else(|| OperatingError::NotRun)?
                }
            };
            if read == FedRead::Collect {
                fed.push(total);
            }
        }
        Ok(
            ryo_execution_domain::domain::operator::types::LogicalCounts {
                fed: fed.into_boxed_slice(),
            },
        )
    }
    #[cfg(any(test, feature = "fixtures"))]
    pub(crate) fn readiness_counts(
        &self,
    ) -> ryo_execution_domain::domain::readiness::types::ReadinessCounts {
        *self.readiness_counts.borrow()
    }
}
