//! Test access to the real selection owner; no mutable backend escapes this module.
use super::*;
use crate::application::file_dag::relations::{PublishedInput, RelationOwner};
use crate::domain::{
    coverage::types::Partial,
    operating::types::OperatorContext,
    stored_selection::types::Selector,
    table_model::types::{Rows, SelectedRows},
};
use ryo_relations::domain::relations::types::{CellKind, ColumnsView, Relation};
use std::{ops::Deref, rc::Rc, sync::Arc};

pub(crate) struct SelectionFixture {
    owner: Rc<RelationOwner>,
}

#[derive(Clone)]
pub(crate) struct SelectedFixtureRows<'f> {
    pub publication: PublishedInput,
    pub rows: SelectedRows<'f>,
}

impl<'f> Deref for SelectedFixtureRows<'f> {
    type Target = SelectedRows<'f>;
    fn deref(&self) -> &Self::Target {
        &self.rows
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SelectionCounters {
    pub scanned: u64,
    pub candidates: u64,
    pub predicate_cells: u64,
    pub builds: u64,
    pub reuses: u64,
    pub refusals: u64,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct SelectionSnapshot {
    pub bytes: usize,
    pub limit: usize,
    pub index_builds: u64,
    pub entries: usize,
    pub selections: usize,
    pub selections_capacity: usize,
    pub owner_metadata_bytes: usize,
    pub optional_bytes: usize,
    pub retained_positions: usize,
    pub retained_positions_capacity: usize,
    pub first_retained_rows_address: Option<usize>,
    pub fanouts: usize,
    pub fanouts_capacity: usize,
    pub descriptor_address: Option<usize>,
    pub prepared_descriptor_address: Option<usize>,
    pub prepared_views_address: Option<usize>,
    pub prepared_selectors: usize,
    pub repeated_discriminator: bool,
    pub counts: SelectionCounters,
}

impl SelectionFixture {
    pub(crate) fn new(
        wanted: &[Relation],
        partials: &[Partial],
        active: Vec<Arc<Selector>>,
        authored: impl IntoIterator<Item = Arc<Selector>>,
        kinds: &'static [CellKind],
    ) -> Result<Self, OperatingError> {
        let owner = Rc::new(RelationOwner::new(
            true,
            crate::outbound::derived::factory::test_port(),
        ));
        let mut cache = super::super::indexes::new(wanted, partials, super::super::indexes::LIMIT)
            .ok_or(OperatingError::ShapeMismatch)?;
        cache.selectors = active;
        let prepared = super::super::selection::prepare_views_with_schemas(
            authored,
            &|_| kinds,
            crate::outbound::derived::factory::test_port(),
        )?;
        super::super::selection::install_views(&mut cache, prepared);
        super::super::indexes::candidate_columns(&mut cache);
        *owner.derived.backend.borrow_mut() = Some(cache);
        Ok(Self { owner })
    }

    pub(crate) fn owner(&self) -> &Rc<RelationOwner> {
        &self.owner
    }

    pub(crate) fn select<'f>(
        &self,
        program: &Arc<Selector>,
        source: ColumnsView<'f>,
        context: &mut OperatorContext<'_>,
    ) -> Result<Option<SelectedFixtureRows<'f>>, OperatingError> {
        let produced =
            self.owner
                .produce_selection(program, &|_| Ok(Rows::Stored(source)), context)?;
        let Some(produced) = produced else {
            return Ok(None);
        };
        let (publication, rows) = self.owner.deliver_selection(produced)?;
        let Rows::Selected(rows) = rows else {
            unreachable!("selected producer")
        };
        Ok(Some(SelectedFixtureRows { publication, rows }))
    }

    pub(crate) fn limit_to_metadata(&self) {
        if let Some(cache) = self.owner.derived.backend.borrow_mut().as_mut() {
            cache.limit = cache.bytes;
        }
    }

    pub(crate) fn ineligible(&self) {
        if let Some(cache) = self.owner.derived.backend.borrow_mut().as_mut() {
            cache.eligible.clear();
        }
    }

    /// Corrupt request-prepared group metadata before any file selection runs.
    pub(crate) fn corrupt_prepared_group_count(&self) {
        let mut backend = self.owner.derived.backend.borrow_mut();
        let cache = backend.as_mut().expect("selection fixture backend");
        let mut prepared = cache
            .selection_views
            .take()
            .expect("prepared selector views");
        Arc::get_mut(&mut prepared)
            .expect("unshared request fixture")
            .groups = 0;
        super::super::selection::install_views(cache, prepared);
    }

    pub(crate) fn reserve_spare_selections(&self, extra: usize) {
        self.owner
            .derived
            .state
            .borrow_mut()
            .entries
            .try_reserve(extra)
            .expect("test spare metadata capacity");
    }

    pub(crate) fn snapshot(&self, descriptor: Option<&Arc<Selector>>) -> SelectionSnapshot {
        let backend = self.owner.derived.backend.borrow();
        let Some(cache) = backend.as_ref() else {
            return SelectionSnapshot::default();
        };
        let counts = &cache.selection_counts;
        let state = self.owner.derived.state.borrow();
        let mut rows = Vec::new();
        let mut predicates = Vec::new();
        let mut add_rows = |value: &Arc<Vec<u32>>| {
            let address = Arc::as_ptr(value) as usize;
            if !rows.iter().any(|(known, _)| *known == address) {
                rows.push((address, value.capacity()));
            }
        };
        for entry in state.entries.values() {
            let Some(retained) = &entry.retained else {
                continue;
            };
            match retained.storage.as_ref() {
                DerivedStorage::RowIds { rows, .. } => {
                    add_rows(rows);
                    if entry._plan.kind == DerivedKind::PredicateMatches {
                        predicates.push(Arc::as_ptr(rows) as usize);
                    }
                }
                DerivedStorage::Postings(postings) => {
                    for entry in postings.iter() {
                        add_rows(&entry.rows);
                    }
                }
                DerivedStorage::Adjacency(graph) => {
                    if let Some(rows) = &graph.identity.population.positions {
                        add_rows(rows);
                    }
                }
                _ => {}
            }
        }
        for fanout in &cache.fanouts {
            for entry in fanout.literals.iter() {
                add_rows(&entry.rows);
            }
        }
        let capacity = rows.iter().map(|(_, capacity)| capacity).sum();
        let optional = self.owner.derived.ledger.borrow().snapshot();
        SelectionSnapshot {
            bytes: cache.bytes,
            limit: cache.limit,
            index_builds: cache.builds,
            entries: cache.entries.len(),
            selections: predicates.len(),
            selections_capacity: state.entries.capacity(),
            owner_metadata_bytes: self.owner.derived.metadata_bytes(),
            optional_bytes: optional.view_optional_bytes,
            retained_positions: optional.optional_units + cache.retained_positions,
            first_retained_rows_address: predicates.first().copied(),
            retained_positions_capacity: capacity,
            fanouts: cache.fanouts.len(),
            fanouts_capacity: cache.fanouts.capacity(),
            descriptor_address: cache.selector_descriptor,
            prepared_descriptor_address: descriptor
                .and_then(|program| {
                    cache
                        .selection_views
                        .as_ref()?
                        .by_selector
                        .get(&(Arc::as_ptr(program) as usize))
                })
                .map(|view| Arc::as_ptr(view) as usize),
            prepared_views_address: cache
                .selection_views
                .as_ref()
                .map(|views| Arc::as_ptr(views) as usize),
            prepared_selectors: cache
                .selection_views
                .as_ref()
                .map_or(0, |views| views.by_selector.len()),
            repeated_discriminator: cache
                .discriminator_columns
                .iter()
                .any(|column| column.repeated),
            counts: SelectionCounters {
                scanned: counts.scanned,
                candidates: counts.candidates,
                predicate_cells: counts.predicate_cells,
                builds: counts.builds,
                reuses: counts.reuses,
                refusals: counts.refusals,
            },
        }
    }
}
