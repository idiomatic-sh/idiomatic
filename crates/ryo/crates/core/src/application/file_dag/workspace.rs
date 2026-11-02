//! The application owns each invocation's mutable allocations and release boundary.
use crate::domain::operator::contract::{
    axis::AxisWorkspace,
    graph::{CycleMarks, DfsWorkspace},
};
use crate::domain::table_cells::types::RowIndex;
use crate::domain::table_model::types::Table;
use crate::domain::table_reach::types::Walk;
use crate::domain::table_scratch::types::{ScratchWord, Stretches};
use ryo_language::domain::relations::types::{Position, Value};
#[derive(Default)]
pub(crate) struct InvocationWorkspace {
    pub relation_owner: Option<std::rc::Rc<super::relations::RelationOwner>>,
    pub publications: Vec<Option<std::sync::Arc<super::relations::RelationResult>>>,
    #[cfg(test)]
    pub observation: crate::domain::operating::types::ExecutionObservation,
    pub downstream_scratch: crate::domain::table_scratch::types::OperatorScratch,
    pub downstream_memo: Vec<crate::domain::operating::types::Memo>,
    pub predicates: crate::domain::table_scratch::types::LazyScratch<
        crate::domain::table_scratch::types::SelectScratch,
    >,
    pub predicate_memo: Vec<crate::domain::operating::types::Memo>,
    pub predicate_names: Option<crate::domain::operating::types::NameCount>,
    pub literal_regex: Option<regex_automata::meta::Regex>,
    pub row: Vec<Value>,
    pub other: Vec<Value>,
    pub text: String,
    pub word: ScratchWord,
    pub stretches: Stretches,
    pub captured: Vec<Value>,
    pub positions: Vec<Position>,
    pub distinct: Table,
    pub seen: hashbrown::HashTable<RowIndex>,
    pub cycle: CycleMarks,
    pub closure: Walk,
    pub dfs: DfsWorkspace,
    pub axis: AxisWorkspace,
}

impl InvocationWorkspace {
    #[cfg(test)]
    pub(crate) fn measured(&self) -> crate::domain::operating::types::ExecutionObservation {
        let mut observed = self.observation;
        observed.regexes = self
            .predicates
            .0
            .get()
            .map_or(0, |state| state.compiled_regexes.0.len())
            + usize::from(self.literal_regex.is_some());
        observed.globs = self
            .predicates
            .0
            .get()
            .map_or(0, |state| state.compiled_globs.len());
        observed.row_capacity = self.row.capacity();
        observed.other_capacity = self.other.capacity();
        observed.closure_visited = self.closure.visited.len().max(self.axis.walk.visited.len());
        observed.dfs_expanded = self.dfs.expanded;
        observed.dfs_examined = self.dfs.examined;
        observed.dfs_frames_len = self.dfs.walk.frames.len();
        observed.dfs_visited_len = self.dfs.walk.visited.len();
        observed
    }
}

impl InvocationWorkspace {
    pub(super) fn capture(&mut self, feed: &dyn super::feed::Feed<'_>, count: usize) {
        self.relation_owner = feed.relation_owner();
        self.publications.clear();
        self.publications.extend(
            (0..count).map(|at| feed.publication(crate::domain::operating::types::InputPlace(at))),
        );
    }
    pub(super) fn normalization(
        &self,
        place: usize,
        rows: crate::domain::table_model::types::Rows<'_>,
    ) -> Option<
        Result<
            Option<std::sync::Arc<crate::domain::table_model::types::Table>>,
            crate::domain::operating::errors::OperatingError,
        >,
    > {
        let (owner, record) = self
            .relation_owner
            .as_ref()
            .zip(self.publications.get(place)?.as_ref())?;
        if !owner.matches(
            record,
            crate::domain::operator::contract::views::BorrowedRows::from(&rows),
        ) {
            return None;
        }
        Some(owner.normalized(record.as_ref(), rows))
    }
    pub(super) fn proven(
        &self,
        place: usize,
        input: crate::domain::operator::contract::views::BorrowedRows<'_>,
    ) -> Result<bool, crate::domain::operating::errors::OperatingError> {
        let Some((owner, record)) = self
            .relation_owner
            .as_ref()
            .zip(self.publications.get(place).and_then(Option::as_ref))
        else {
            return Ok(false);
        };
        if !owner.matches(record, input) {
            return Ok(false);
        }
        Ok(owner.inspect(record.as_ref(),input)?==crate::domain::table_model::collections::representations::types::PhysicalShape::Complete)
    }
    pub(super) fn validate(
        &self,
        place: usize,
        input: crate::domain::operator::contract::views::BorrowedRows<'_>,
    ) -> Result<(), crate::domain::operating::errors::OperatingError> {
        if self.proven(place, input)? {
            Ok(())
        } else {
            input.validate()
        }
    }
}
