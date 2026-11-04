//! One file generation owns representation descriptors and lazy admission proofs.
use super::derived::owner::PopulationId;
use crate::domain::operator::contract::views::BorrowedRows;
use crate::domain::table_model::collections::representations::{
    publication::PublishedRelation,
    service::{kind, registered},
    types::{PhysicalShape, RepresentationKind},
};
use crate::domain::table_model::{
    service::materialize,
    types::{Rows, SelectedRows, StoredSelection},
};
use crate::domain::{file_program::types::NodeId, operating::errors::OperatingError};
use ryo_relations::domain::relations::types::{CellKind, Relation};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    sync::{Arc, OnceLock},
};
mod accounting;
mod admission;
mod external;
mod handle;
pub(crate) use handle::{PublishedInput, RetainedPublication};
mod identity;
mod produced;
mod selected_access;
mod selected_delivery;
pub(super) use {admission::AdmittedInput, produced::ProducedSelection};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct SourceDescriptor {
    relation: Relation,
    kind: RepresentationKind,
    kinds: Vec<CellKind>,
    rows: usize,
    storage: Vec<(usize, usize)>,
    projection: Vec<ryo_values::domain::plan_parts::types::ColumnIndex>,
    source_kinds: Vec<CellKind>,
}
#[derive(Debug)]
enum RelationBody {
    Source(SourceDescriptor),
    Selected {
        source: Arc<RelationResult>,
        selection: Arc<StoredSelection>,
    },
    Computed(Arc<Table>),
}
mod packed;
pub(crate) use packed::PackedPublication;
mod record;
mod number;
pub(crate) use record::RelationResult;

struct SourceViews {
    population: SourceDescriptor,
    canonical_population: PopulationId,
    views: HashMap<SourceDescriptor, PublishedInput>,
}

pub(crate) struct RelationOwner {
    #[cfg(test)]
    pub(super) identity_checks: Cell<u64>,
    pub(super) derived: super::derived::owner::DerivedOwner,
    next_population: Cell<usize>,
    producer_populations: RefCell<HashMap<usize, (usize, PopulationId)>>,
    generation: RefCell<Arc<()>>,
    stable: bool,
    sources: RefCell<HashMap<Relation, SourceViews>>,
    max_views: usize,
    views: Cell<usize>,
    pub checks: Cell<u64>,
    pub reuses: Cell<u64>,
}
impl RelationOwner {
    pub(super) fn for_scope(
        _permit: super::scope::ScopePermit,
        stable: bool,
        views: usize,
        semantic: Arc<dyn crate::outbound::derived::interface::DerivedPort>,
    ) -> Self {
        Self::with_limits(stable, views,
            views.saturating_mul(ryo_derived::domain::contract::types::DerivedKind::ALL.len()), semantic)
    }
    #[cfg(test)]
    pub(in crate::application) fn new(stable: bool, semantic: Arc<dyn crate::outbound::derived::interface::DerivedPort>) -> Self {
        Self::for_scope(super::scope::ScopePermit::fixture(), stable, 1024, semantic)
    }
    #[cfg(test)]
    pub(in crate::application) fn for_request(stable: bool, views: usize, semantic: Arc<dyn crate::outbound::derived::interface::DerivedPort>) -> Self {
        Self::for_scope(super::scope::ScopePermit::fixture(), stable, views, semantic)
    }
    fn with_limits(
        stable: bool,
        max_views: usize,
        max_derived: usize,
        semantic: Arc<dyn crate::outbound::derived::interface::DerivedPort>,
    ) -> Self {
        Self {
            #[cfg(test)]
            identity_checks: Cell::new(0),
            derived: super::derived::owner::DerivedOwner::new(max_derived, semantic),
            next_population: Cell::new(0),
            producer_populations: RefCell::new(HashMap::new()),
            generation: RefCell::new(Arc::new(())),
            stable,
            sources: RefCell::new(HashMap::new()),
            max_views,
            views: Cell::new(0),
            checks: Cell::new(0),
            reuses: Cell::new(0),
        }
    }
    pub(super) fn source(
        &self,
        relation: Relation,
        rows: &Rows<'_>,
    ) -> Result<PublishedInput, OperatingError> {
        self.source_borrowed(relation, BorrowedRows::from(rows))
    }
    fn source_borrowed(
        &self,
        relation: Relation,
        raw: BorrowedRows<'_>,
    ) -> Result<PublishedInput, OperatingError> {
        let population = match raw {
            BorrowedRows::Selected { source, .. } => BorrowedRows::Stored(source),
            other => other,
        };
        if let Some(found) = self.sources.borrow().get(&relation) {
            if !identity::matches(&found.population, population) {
                return Err(OperatingError::NotRead);
            }
            if let Some(view) = found.views.get(&identity::describe(relation, raw)) {
                return Ok(view.clone());
            }
        }
        let result = Arc::new(RelationResult {
            population: self.allocate_population()?,
            producer: OnceLock::new(),
            generation: Arc::clone(&self.generation.borrow()),
            kind: kind(raw),
            body: RelationBody::Source(identity::describe(relation, raw)),
            inspected: OnceLock::new(),
            normalized: OnceLock::new(),
        });
        let result = self.publication(result)?;
        if self.stable && self.views.get() < self.max_views {
            let mut sources = self.sources.borrow_mut();
            let entry = match sources.entry(relation) {
                std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
                std::collections::hash_map::Entry::Vacant(entry) => entry.insert(SourceViews {
                    population: identity::describe(relation, population),
                    // Graph aliases share a separate admitted ID. Pinning a
                    // graph source must not promote unrelated ordinary views
                    // keyed by the raw relation's primary population.
                    canonical_population: PopulationId(self.allocate_population()?),
                    views: HashMap::new(),
                }),
            };
            entry
                .views
                .insert(identity::describe(relation, raw), result.clone());
            self.views.set(self.views.get() + 1);
        }
        Ok(result)
    }
    pub(super) fn computed(&self, table: Table) -> Result<PublishedInput, OperatingError> {
        let input = BorrowedRows::Computed(&table);
        let contract = input.contract()?;
        registered(contract.kind).inspect(&contract)?;
        self.publication(Arc::new(RelationResult {
            population: self.allocate_population()?,
            producer: OnceLock::new(),
            generation: Arc::clone(&self.generation.borrow()),
            kind: RepresentationKind::Computed,
            body: RelationBody::Computed(Arc::new(table)),
            inspected: OnceLock::from(Ok(PhysicalShape::Complete)),
            normalized: OnceLock::new(),
        }))
    }
    pub(super) fn publish_selection(
        &self,
        produced: ProducedSelection<'_>,
    ) -> Result<PublishedInput, OperatingError> {
        let ProducedSelection {
            source,
            selection,
            resources,
            ..
        } = produced;
        self.generation(&source)?;
        self.publication_with_resources(
            Arc::new(RelationResult {
                population: self.allocate_population()?,
                producer: OnceLock::new(),
                generation: Arc::clone(&self.generation.borrow()),
                kind: RepresentationKind::Selected,
                body: RelationBody::Selected {
                    source: Arc::clone(source.record()),
                    selection,
                },
                inspected: OnceLock::from(Ok(PhysicalShape::Complete)),
                normalized: OnceLock::new(),
            }),
            resources,
        )
    }
    pub(super) fn stable(&self) -> bool {
        self.stable
    }
    pub(super) fn produce_selection<'f>(
        &self,
        program: &Arc<crate::domain::stored_selection::types::Selector>,
        read: &dyn Fn(Relation) -> Result<Rows<'f>, OperatingError>,
        context: &mut crate::domain::operating::types::OperatorContext<'_>,
    ) -> Result<Option<ProducedSelection<'f>>, OperatingError> {
        produced::select(self, program, read, context)
    }
    /// Explicitly ends the immutable resident-population lease before release or refill.
    pub(super) fn release_population(&self) {
        self.derived.clear();
        self.producer_populations.borrow_mut().clear();
        self.sources.borrow_mut().clear();
        self.views.set(0);
        *self.generation.borrow_mut() = Arc::new(());
    }
    fn private<'a>(
        &self,
        result: &'a dyn PublishedRelation,
    ) -> Result<&'a RelationResult, OperatingError> {
        let record = result
            .as_any()
            .downcast_ref::<RelationResult>()
            .ok_or(OperatingError::NotRead)?;
        self.generation(record)?;
        Ok(record)
    }

    fn generation(&self, result: &RelationResult) -> Result<(), OperatingError> {
        if Arc::ptr_eq(&self.generation.borrow(), &result.generation) {
            Ok(())
        } else {
            Err(OperatingError::NotRead)
        }
    }
    fn matches_current(&self, result: &RelationResult, input: BorrowedRows<'_>) -> bool {
        #[cfg(test)]
        self.identity_checks.set(self.identity_checks.get() + 1);
        // publish_selection verifies its source epoch; private fields preserve
        // that invariant, so the selected chain shares the checked outer epoch.
        (result.normalized.get().is_some_and(|table|matches!(input,BorrowedRows::Computed(actual)if std::ptr::eq(actual,table.as_ref())))||match &result.body{
   RelationBody::Source(source)=>identity::matches(source,input),
   RelationBody::Computed(table)=>matches!(input,BorrowedRows::Computed(actual)if std::ptr::eq(actual,table.as_ref())),
   RelationBody::Selected{selection,source}=>matches!(input,BorrowedRows::Selected{source:actual,positions,columns,kinds}if std::ptr::eq(positions,selection.rows.as_slice())&&columns==selection.columns&&kinds==selection.kinds&&self.matches_current(source,BorrowedRows::Stored(actual))),
  })
    }
    pub(super) fn resolve<'r, 'f>(
        &self,
        result: &'r dyn PublishedRelation,
        read: &dyn Fn(Relation) -> Result<Rows<'f>, OperatingError>,
    ) -> Result<Rows<'r>, OperatingError>
    where
        'f: 'r,
    {
        let result = self.private(result)?;
        match &result.body {
            RelationBody::Source(source) => {
                let rows = read(source.relation)?;
                if !identity::matches(source, BorrowedRows::from(&rows)) {
                    return Err(OperatingError::NotRead);
                }
                Ok(rows)
            }
            RelationBody::Computed(table) => Ok(Rows::Computed(table)),
            RelationBody::Selected { source, selection } => {
                let Rows::Stored(columns) = self.resolve(source.as_ref(), read)? else {
                    return Err(OperatingError::WrongOperation);
                };
                Ok(Rows::Selected(SelectedRows {
                    source: columns,
                    selection: Arc::clone(selection),
                }))
            }
        }
    }
    pub(super) fn normalized(
        &self,
        result: &PublishedInput,
        rows: Rows<'_>,
    ) -> Result<Option<Arc<Table>>, OperatingError> {
        let _checked = self.normalize(result, rows)?;
        Ok(self.private(result.as_ref())?.normalized.get().cloned())
    }
    /// The caller invokes this only at its already declared physical demand gate.
    pub(super) fn inspect(
        &self,
        result: &dyn PublishedRelation,
        input: BorrowedRows<'_>,
    ) -> Result<PhysicalShape, OperatingError> {
        let result = self.private(result)?;
        if !self.matches_current(result, input) {
            return Err(OperatingError::NotRead);
        }
        self.inspect_current(result, input)
    }
    pub(super) fn admit<'a>(
        &self,
        result: &dyn PublishedRelation,
        input: BorrowedRows<'a>,
    ) -> Result<AdmittedInput<'a>, OperatingError> {
        let result = self.private(result)?;
        if !self.matches_current(result, input) {
            return Err(OperatingError::NotRead);
        }
        let shape = self.inspect_current(result, input)?;
        Ok(AdmittedInput {
            rows: input,
            shape,
            backing_population: self.backing_population(result),
            backing_relation: Self::backing_relation(result),
            canonical_backing: Self::backing_relation(result)
                .is_some_and(|relation| self.sources.borrow().contains_key(&relation)),
            origin: (
                self.result_population(result),
                match &result.body {
                    RelationBody::Source(source) => Some(source.relation),
                    _ => None,
                },
                Arc::clone(&result.generation),
            ),
        })
    }

    fn inspect_current(
        &self,
        result: &RelationResult,
        input: BorrowedRows<'_>,
    ) -> Result<PhysicalShape, OperatingError> {
        if result.normalized.get().is_some_and(|table|matches!(input,BorrowedRows::Computed(actual)if std::ptr::eq(actual,table.as_ref()))){self.reuses.set(self.reuses.get()+1);return Ok(PhysicalShape::Complete)}
        if result.inspected.get().is_some() {
            self.reuses.set(self.reuses.get() + 1);
        }
        let outcome = result
            .inspected
            .get_or_init(|| {
                self.checks.set(self.checks.get() + 1);
                let contract = input.contract()?;
                crate::application::file_dag::route_counts::observe(
                    "representation_inspect",
                    crate::application::file_dag::route_counts::representation(contract.kind),
                    || registered(contract.kind).inspect(&contract),
                )
            })
            .clone();
        if outcome != Ok(PhysicalShape::Complete) {
            if let RelationBody::Source(source) = &result.body {
                if let Some(entry) = self.sources.borrow_mut().get_mut(&source.relation) {
                    let before = entry.views.len();
                    entry
                        .views
                        .retain(|_, view| !std::ptr::eq(view.as_ref(), result));
                    self.views
                        .set(self.views.get().saturating_sub(before - entry.views.len()));
                }
            }
        }
        outcome
    }
    pub(super) fn normalize<'r, 'f>(
        &self,
        publication: &'r PublishedInput,
        rows: Rows<'f>,
    ) -> Result<Rows<'r>, OperatingError>
    where
        'f: 'r,
    {
        let result = self.private(publication.as_ref())?;
        match self.inspect(result, BorrowedRows::from(&rows))? {
            PhysicalShape::Complete => Ok(rows),
            PhysicalShape::ReadablePhysicalFallback => {
                let table = result.normalized.get_or_init(|| {
                    let mut table = Table::default();
                    materialize(&rows, &mut table, &mut Vec::new());
                    Arc::new(table)
                });
                if !publication.normalization_charged() {
                    let lease = self.derived.required_lease(&[Self::table_backing(table)])?;
                    publication.hold_normalization(lease);
                }
                Ok(Rows::Computed(table))
            }
        }
    }
}
pub(super) use accounting::bytes;

#[cfg(test)]
mod owner_test;

impl RelationOwner {
    fn allocate_population(&self) -> Result<usize, OperatingError> {
        let id = self.next_population.get();
        self.next_population
            .set(id.checked_add(1).ok_or(OperatingError::ShapeMismatch)?);
        Ok(id)
    }
    /// One canonical DAG producer has one population identity across rules and safe recomputation.
    pub(super) fn bind_producer(
        &self,
        node: NodeId,
        result: &RelationResult,
    ) -> Result<(), OperatingError> {
        self.generation(result)?;
        if matches!(result.body, RelationBody::Source(_)) || result.producer.get().is_some() {
            self.producer_populations
                .borrow_mut()
                .entry(node.0)
                .or_insert((
                    self.result_population(result).0,
                    self.backing_population(result),
                ));
            return Ok(());
        }
        let mut known = self.producer_populations.borrow_mut();
        let (id, _) = *known
            .entry(node.0)
            .or_insert((result.population, self.backing_population(result)));
        let _installed = result.producer.set(id);
        Ok(())
    }
    fn result_population(&self, result: &RelationResult) -> PopulationId {
        PopulationId(*result.producer.get().unwrap_or(&result.population))
    }
    fn backing_relation(result: &RelationResult) -> Option<Relation> {
        match &result.body {
            RelationBody::Source(source) => Some(source.relation),
            RelationBody::Selected { source, .. } => Self::backing_relation(source),
            RelationBody::Computed(_) => None,
        }
    }
    fn backing_population(&self, result: &RelationResult) -> PopulationId {
        match &result.body {
            RelationBody::Source(source) => {
                self.sources.borrow().get(&source.relation).map_or_else(
                    || self.result_population(result),
                    |source| source.canonical_population,
                )
            }
            RelationBody::Selected { source, .. } => self.backing_population(source),
            RelationBody::Computed(_) => self.result_population(result),
        }
    }
    pub(super) fn pin_derived(&self, result: &RelationResult) {
        let population = self.result_population(result);
        self.derived.pin(population);
        let backing = self.backing_population(result);
        if backing != population {
            self.derived.pin(backing);
        }
    }
    /// Global consumer cancellation need not retain the producer payload.
    /// A generation clear has already released all pins and removed this map.
    pub(super) fn release_derived_producer(
        &self,
        node: crate::domain::file_program::types::NodeId,
    ) {
        if let Some((population, backing)) =
            self.producer_populations.borrow().get(&node.0).copied()
        {
            let population = PopulationId(population);
            self.derived.release(population);
            if backing != population {
                self.derived.release(backing);
            }
        }
    }
    pub(super) fn release_derived(&self, result: &RelationResult) {
        let population = self.result_population(result);
        self.derived.release(population);
        let backing = self.backing_population(result);
        if backing != population {
            self.derived.release(backing);
        }
    }
}

use ryo_values::domain::rows::types::Table;
