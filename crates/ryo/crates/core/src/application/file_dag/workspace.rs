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
    pub derived_local: super::derived::owner::DerivedOwner,
    pub derived_scratch: std::cell::RefCell<crate::domain::derived_views::types::DerivedScratch>,
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
    pub(super) fn reclaim_derived(
        &self,
        cache: &mut crate::domain::stored_indexes::types::FileIndexes,
        cost: usize,
    ) {
        self.derived_local.reclaim_optional(cache, cost);
        if let Some(owner) = &self.relation_owner {
            owner.derived.reclaim_optional(cache, cost);
        }
    }
    pub(super) fn derived_counts(&self) -> (u64, u64) {
        let (mut builds, mut hits, _) = self.derived_local.counts();
        if let Some(owner) = &self.relation_owner {
            let (shared_builds, shared_hits, _) = owner.derived.counts();
            builds += shared_builds;
            hits += shared_hits;
        }
        (builds, hits)
    }

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

impl InvocationWorkspace {
    pub(super) fn acquire_rows(
        &self,
        use_: &crate::domain::derived_views::types::PreparedViewUse,
        inputs: &[crate::domain::operator::contract::views::BorrowedRows<'_>],
        context: &mut crate::domain::operating::types::OperatorContext<'_>,
    ) -> Result<
        std::sync::Arc<crate::domain::derived_views::types::DerivedStorage>,
        crate::domain::operating::errors::OperatingError,
    > {
        if !(1..=2).contains(&use_.len) {
            return Err(crate::domain::operating::errors::OperatingError::WrongOperation);
        }
        let mut rows = [crate::domain::operator::contract::views::BorrowedRows::Computed(
            &EMPTY_DERIVED_INPUT,
        ); 2];
        for at in 0..usize::from(use_.len) {
            let place = use_.inputs[at];
            rows[at] = *inputs.get(place).ok_or(
                crate::domain::operating::errors::OperatingError::MissingInput {
                    position: place + 1,
                },
            )?;
        }
        self.acquire_derived(
            use_,
            inputs,
            super::derived::owner::ViewContext::default(),
            crate::domain::derived_views::types::DerivedInputs::Rows(
                &rows[..usize::from(use_.len)],
            ),
            context.stored_indexes.as_deref_mut(),
        )
    }
    pub(super) fn acquire_derived(
        &self,
        use_: &crate::domain::derived_views::types::PreparedViewUse,
        inputs: &[crate::domain::operator::contract::views::BorrowedRows<'_>],
        identity: super::derived::owner::ViewContext,
        supplied: crate::domain::derived_views::types::DerivedInputs<'_, '_>,
        cache: Option<&mut crate::domain::stored_indexes::types::FileIndexes>,
    ) -> Result<
        std::sync::Arc<crate::domain::derived_views::types::DerivedStorage>,
        crate::domain::operating::errors::OperatingError,
    > {
        use super::derived::owner::PopulationId;
        if !(1..=2).contains(&use_.len) {
            return Err(crate::domain::operating::errors::OperatingError::WrongOperation);
        }
        let mut populations = [PopulationId(0); 2];
        let mut shared = self.relation_owner.is_some();
        for at in 0..usize::from(use_.len) {
            let place = use_.inputs[at];
            let input = *inputs.get(place).ok_or(
                crate::domain::operating::errors::OperatingError::MissingInput {
                    position: place + 1,
                },
            )?;
            self.validate(place, input)?;
            if let Some((owner, publication)) = self
                .relation_owner
                .as_ref()
                .zip(self.publications.get(place).and_then(Option::as_ref))
            {
                if owner.matches(publication, input) {
                    populations[at] = owner.population_id(publication, input)?;
                    continue;
                }
            }
            shared = false;
            populations[at] = PopulationId(place);
        }
        let owner = if shared {
            &self
                .relation_owner
                .as_ref()
                .expect("admitted owner")
                .derived
        } else {
            &self.derived_local
        };
        let stored = if use_.plan.kind == crate::domain::derived_views::types::DerivedKind::Equality
            && use_.len == 1
        {
            let place = use_.inputs[0];
            match inputs[place] {
                crate::domain::operator::contract::views::BorrowedRows::Stored(source) => {
                    let relation = use_.relations[0].or_else(|| {
                        self.relation_owner
                            .as_ref()
                            .zip(self.publications.get(place).and_then(Option::as_ref))
                            .and_then(|(owner, publication)| {
                                owner.source_relation(publication, inputs[place])
                            })
                    });
                    relation.map(|relation| (relation, source))
                }
                _ => None,
            }
        } else {
            None
        };
        owner.acquire_stored(
            &use_.plan,
            &populations[..usize::from(use_.len)],
            identity,
            supplied,
            &mut self.derived_scratch.borrow_mut(),
            stored,
            cache,
        )
    }
}

static EMPTY_DERIVED_INPUT: Table = Table {
    kinds: Vec::new(),
    cells: Vec::new(),
    count: crate::domain::table_model::types::RowCount(0),
};
