//! The application owns the declared inventory, preparation state and sealing.
use super::{
    errors::PreparationError,
    types::{Occurrence, PreparedProject},
};
use crate::{application::cross_file::errors::CrossError, domain::answering::types::Todo};
use ryo_execution::inbound::use_cases::execution::types::RuleIndex;
use std::sync::Arc;

/// A stable occurrence, including when several declarations name the same rule.
#[cfg(test)]
#[derive(Clone)]
pub(crate) struct Slot {
    owner: Arc<()>,
    occurrence: Occurrence,
}
enum State {
    Pending,
    Ready(PreparedProject),
    Failed(CrossError),
}
struct Entry {
    todo: Todo,
    state: State,
}

pub(crate) struct Preparation {
    #[cfg(test)]
    owner: Arc<()>,
    entries: Box<[Entry]>,
}
/// Only this owner can issue a batch: every immutable declaration is accounted.
pub(crate) struct SealedBatch {
    declared: Arc<[Todo]>,
    entries: Box<[(Todo, Result<PreparedProject, CrossError>)]>,
}
impl Preparation {
    pub(crate) fn ordinary(todo: &[Todo]) -> Self {
        Self::declared(todo.iter().cloned())
    }
    /// Select each authored occurrence in batch order. Neither repeated batch
    /// entries nor repeated selected Todo occurrences are deduplicated by rule.
    pub(crate) fn named(batch: &[RuleIndex], todo: &[Todo]) -> Self {
        Self::declared(
            batch
                .iter()
                .flat_map(|rule| todo.iter().filter(move |todo| todo.rule == *rule).cloned()),
        )
    }
    fn declared(todo: impl Iterator<Item = Todo>) -> Self {
        Self {
            #[cfg(test)]
            owner: Arc::new(()),
            entries: todo
                .map(|todo| Entry {
                    todo,
                    state: State::Pending,
                })
                .collect(),
        }
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    #[cfg(test)]
    pub(crate) fn slots(&self) -> Vec<Slot> {
        (0..self.entries.len())
            .map(|at| Slot {
                owner: Arc::clone(&self.owner),
                occurrence: Occurrence(at),
            })
            .collect()
    }
    #[cfg(test)]
    pub(crate) fn account(
        &mut self,
        slot: Slot,
        result: Result<PreparedProject, CrossError>,
    ) -> Result<(), PreparationError> {
        if !Arc::ptr_eq(&self.owner, &slot.owner) {
            return Err(PreparationError::Foreign);
        }
        let entry = self
            .entries
            .get_mut(slot.occurrence.0)
            .ok_or(PreparationError::Foreign)?;
        if !matches!(entry.state, State::Pending) {
            return Err(PreparationError::Repeated(slot.occurrence));
        }
        entry.state = match result {
            Ok(ready) => State::Ready(ready),
            Err(error) => State::Failed(error),
        };
        Ok(())
    }
    /// The owner drives preparation of the whole declared inventory. Callers
    /// supply resources or the actual error; they cannot filter pending slots.
    pub(crate) fn prepare_all(
        mut self,
        mut prepare: impl FnMut(Occurrence, &Todo) -> Result<PreparedProject, CrossError>,
    ) -> Self {
        for (at, entry) in self.entries.iter_mut().enumerate() {
            if matches!(entry.state, State::Pending) {
                entry.state = match prepare(Occurrence(at), &entry.todo) {
                    Ok(ready) => State::Ready(ready),
                    Err(error) => State::Failed(error),
                };
            }
        }
        self
    }
    pub(crate) fn seal(self) -> Result<SealedBatch, PreparationError> {
        if let Some(at) = self
            .entries
            .iter()
            .position(|entry| matches!(entry.state, State::Pending))
        {
            return Err(PreparationError::Missing(Occurrence(at)));
        }
        let declared = self
            .entries
            .iter()
            .map(|entry| entry.todo.clone())
            .collect();
        let entries = self
            .entries
            .into_vec()
            .into_iter()
            .map(|entry| {
                let result = match entry.state {
                    State::Ready(ready) => Ok(ready),
                    State::Failed(error) => Err(error),
                    State::Pending => {
                        unreachable!("pending slots were refused before issuing the batch")
                    }
                };
                (entry.todo, result)
            })
            .collect();
        Ok(SealedBatch { declared, entries })
    }
}
impl SealedBatch {
    /// The same immutable manifest survives refused execution for per-rule retirement.
    pub(in crate::application::cross_file) fn declarations(&self) -> Arc<[Todo]> {
        Arc::clone(&self.declared)
    }
    pub(in crate::application::cross_file) fn into_entries(
        self,
    ) -> Box<[(Todo, Result<PreparedProject, CrossError>)]> {
        self.entries
    }
}
