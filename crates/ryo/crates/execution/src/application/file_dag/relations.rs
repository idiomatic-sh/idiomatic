//! One file generation owns representation descriptors and lazy admission proofs.
use super::derived::population::types::PopulationId;
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
use crate::domain::{executable_program::types::RegionId, operating::errors::OperatingError};
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
pub(crate) use handle::PublishedInput;
mod identity;
pub(super) mod positioned;
mod selected_access;
pub(super) mod selection;
pub(super) use admission::AdmittedInput;

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
    Positioned {
        source: Arc<RelationResult>,
        positions: Arc<crate::domain::positions::types::Positions>,
        shape: Arc<crate::domain::lowering::types::LoweredShape>,
        kinds: Arc<[CellKind]>,
    },
    Computed(Arc<Table>),
    Compact(Arc<ryo_values::domain::table_words::types::CompactTable>),
}
mod packed;
pub(crate) use packed::PackedPublication;
mod number;
pub(in crate::application::file_dag) use number::AdmittedNumber;
mod record;
pub(crate) use record::RelationResult;

struct SourceViews {
    population: SourceDescriptor,
    canonical_population: PopulationId,
    views: HashMap<SourceDescriptor, PublishedInput>,
}

/// Read-only revocation witness. Only the population owner can invalidate it;
/// semantic callbacks never receive a mutable atomic or a revival operation.
pub(in crate::application::file_dag) struct GenerationLease {
    live: std::sync::atomic::AtomicBool,
}
impl GenerationLease {
    fn new() -> Self {
        Self {
            live: std::sync::atomic::AtomicBool::new(true),
        }
    }
    fn invalidate(&self) {
        self.live.store(false, std::sync::atomic::Ordering::Release);
    }
    pub(in crate::application::file_dag) fn is_live(&self) -> bool {
        self.live.load(std::sync::atomic::Ordering::Acquire)
    }
}
/// A typed invocation namespace; publications retain their actual population after it ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in crate::application::file_dag) enum ProducerScope {
    File,
    Project(PopulationId),
}
pub(crate) struct RelationOwner {
    #[cfg(test)]
    pub(super) identity_checks: Cell<u64>,
    pub(super) derived: super::derived::owner::DerivedOwner,
    next_population: Cell<usize>,
    producer_populations: RefCell<HashMap<(ProducerScope, usize), (usize, PopulationId)>>,
    generation: RefCell<Arc<()>>,
    generation_live: RefCell<Arc<GenerationLease>>,
    stable: bool,
    sources: RefCell<HashMap<Relation, SourceViews>>,
    max_views: usize,
    views: Cell<usize>,
    pub checks: Cell<u64>,
    pub reuses: Cell<u64>,
}
impl RelationOwner {
    pub(in crate::application::file_dag) fn publication_kinds<'a>(
        &self,
        publication: &'a PublishedInput,
    ) -> Result<&'a [CellKind], OperatingError> {
        let record = publication.as_ref();
        self.generation(record)?;
        Ok(match &record.body {
            RelationBody::Source(source) => &source.kinds,
            RelationBody::Selected { selection, .. } => &selection.kinds,
            RelationBody::Positioned { kinds, .. } => kinds,
            RelationBody::Computed(table) => &table.kinds,
            RelationBody::Compact(table) => {
                ryo_values::domain::table_words::service::compact_kinds(table)
            }
        })
    }
    pub(in crate::application::file_dag) fn generation_guard(&self) -> Arc<()> {
        Arc::clone(&self.generation.borrow())
    }
    pub(in crate::application::file_dag) fn generation_liveness(&self) -> Arc<GenerationLease> {
        Arc::clone(&self.generation_live.borrow())
    }
    pub(in crate::application::file_dag) fn matches_generation(
        &self,
        generation: &Arc<()>,
    ) -> bool {
        Arc::ptr_eq(generation, &self.generation.borrow())
    }

    pub(super) fn for_scope(
        _permit: super::scope::ScopePermit,
        stable: bool,
        views: usize,
        semantic: Arc<dyn crate::outbound::derived::interface::DerivedPort>,
    ) -> Self {
        Self::with_limits(
            stable,
            views,
            views.saturating_mul(ryo_derived::domain::contract::types::DerivedKind::ALL.len()),
            semantic,
        )
    }
    #[cfg(test)]
    pub(in crate::application) fn new(
        stable: bool,
        semantic: Arc<dyn crate::outbound::derived::interface::DerivedPort>,
    ) -> Self {
        Self::for_scope(super::scope::ScopePermit::fixture(), stable, 1024, semantic)
    }
    #[cfg(test)]
    pub(in crate::application) fn for_request(
        stable: bool,
        views: usize,
        semantic: Arc<dyn crate::outbound::derived::interface::DerivedPort>,
    ) -> Self {
        Self::for_scope(
            super::scope::ScopePermit::fixture(),
            stable,
            views,
            semantic,
        )
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
            generation_live: RefCell::new(Arc::new(GenerationLease::new())),
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
            producer_scope: OnceLock::new(),
            generation: Arc::clone(&self.generation.borrow()),

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
    /// Custody of a supplied compact source; no kernel permit is issued here.
    pub(super) fn source_compact(
        &self,
        table: Arc<ryo_values::domain::table_words::types::CompactTable>,
    ) -> Result<PublishedInput, OperatingError> {
        self.publication(Arc::new(RelationResult {
            population: self.allocate_population()?,
            producer: OnceLock::new(),
            producer_scope: OnceLock::new(),
            generation: Arc::clone(&self.generation.borrow()),
            body: RelationBody::Compact(table),
            inspected: OnceLock::new(),
            normalized: OnceLock::new(),
        }))
    }
    /// Custody of a supplied flat source, preserving exact geometry for first-borrow inspection.
    pub(super) fn source_shared_computed(
        &self,
        table: Arc<Table>,
    ) -> Result<PublishedInput, OperatingError> {
        self.publication(Arc::new(RelationResult {
            population: self.allocate_population()?,
            producer: OnceLock::new(),
            producer_scope: OnceLock::new(),
            generation: Arc::clone(&self.generation.borrow()),
            body: RelationBody::Computed(table),
            inspected: OnceLock::new(),
            normalized: OnceLock::new(),
        }))
    }
    pub(super) fn computed(&self, table: Table) -> Result<PublishedInput, OperatingError> {
        let input = BorrowedRows::Computed(&table);
        let contract = input.contract()?;
        registered(contract.kind).inspect(&contract)?;
        self.publication(Arc::new(RelationResult {
            population: self.allocate_population()?,
            producer: OnceLock::new(),
            producer_scope: OnceLock::new(),
            generation: Arc::clone(&self.generation.borrow()),

            body: RelationBody::Computed(Arc::new(table)),
            inspected: OnceLock::from(Ok(PhysicalShape::Complete)),
            normalized: OnceLock::new(),
        }))
    }
    fn position_source(
        &self,
        publication: &PublishedInput,
    ) -> Result<Arc<RelationResult>, OperatingError> {
        self.generation(publication)?;
        match &publication.body {
            RelationBody::Positioned { source, .. } => Ok(Arc::clone(source)),
            RelationBody::Source(source) if source.kind == RepresentationKind::Tree => {
                Ok(Arc::clone(publication.record()))
            }
            _ => Err(OperatingError::WrongOperation),
        }
    }
    pub(super) fn stable(&self) -> bool {
        self.stable
    }
    /// Explicitly ends the immutable resident-population lease before release or refill.
    pub(super) fn release_population(&self) {
        self.derived.clear();
        self.producer_populations.borrow_mut().clear();
        self.sources.borrow_mut().clear();
        self.views.set(0);
        self.generation_live.borrow().invalidate();
        *self.generation_live.borrow_mut() = Arc::new(GenerationLease::new());
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
   RelationBody::Compact(table)=>matches!(input,BorrowedRows::Compact(actual)if std::ptr::eq(actual,table.as_ref())),
   RelationBody::Positioned{source,positions,shape,kinds}=>matches!(input,BorrowedRows::Positioned(actual)if std::ptr::eq(actual.positions,positions.as_ref())&&std::ptr::eq(actual.shape,shape.as_ref())&&actual.kinds==kinds.as_ref()&&self.matches_current(source,BorrowedRows::Tree(actual.tree))),
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
            RelationBody::Compact(table) => Ok(Rows::Compact(table)),
            RelationBody::Positioned {
                source,
                positions,
                shape,
                kinds,
            } => {
                let Rows::Tree(tree) = self.resolve(source.as_ref(), read)? else {
                    return Err(OperatingError::WrongOperation);
                };
                Ok(Rows::Positioned(
                    crate::domain::table_model::types::PositionedRows {
                        tree,
                        positions,
                        shape,
                        kinds,
                    },
                ))
            }
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

#[cfg(test)]
mod owner_test;

impl RelationOwner {
    pub(in crate::application::file_dag) fn project_identity(
        &self,
    ) -> Result<PopulationId, OperatingError> {
        self.allocate_population().map(PopulationId)
    }
    fn allocate_population(&self) -> Result<usize, OperatingError> {
        let id = self.next_population.get();
        self.next_population
            .set(id.checked_add(1).ok_or(OperatingError::ShapeMismatch)?);
        Ok(id)
    }
    /// One canonical DAG producer has one population identity across rules and safe recomputation.
    pub(super) fn bind_producer(
        &self,
        scope: ProducerScope,
        node: RegionId,
        result: &RelationResult,
    ) -> Result<(), OperatingError> {
        self.generation(result)?;
        match result.producer_scope.get() {
            Some(existing) if *existing != scope => return Err(OperatingError::WrongBinding),
            Some(_) => {}
            None => result
                .producer_scope
                .set(scope)
                .map_err(|_| OperatingError::WrongBinding)?,
        }
        if matches!(result.body, RelationBody::Source(_)) || result.producer.get().is_some() {
            self.producer_populations
                .borrow_mut()
                .entry((scope, node.0))
                .or_insert((
                    self.result_population(result).0,
                    self.backing_population(result),
                ));
            return Ok(());
        }
        let mut known = self.producer_populations.borrow_mut();
        let (id, _) = *known
            .entry((scope, node.0))
            .or_insert((result.population, self.backing_population(result)));
        result
            .producer
            .set(id)
            .map_err(|_| OperatingError::WrongBinding)?;
        Ok(())
    }
    fn result_population(&self, result: &RelationResult) -> PopulationId {
        PopulationId(*result.producer.get().unwrap_or(&result.population))
    }
    fn backing_relation(result: &RelationResult) -> Option<Relation> {
        match &result.body {
            RelationBody::Source(source) => Some(source.relation),
            RelationBody::Selected { source, .. } | RelationBody::Positioned { source, .. } => {
                Self::backing_relation(source)
            }
            RelationBody::Computed(_) | RelationBody::Compact(_) => None,
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
            RelationBody::Selected { source, .. } | RelationBody::Positioned { source, .. } => {
                self.backing_population(source)
            }
            RelationBody::Computed(_) | RelationBody::Compact(_) => self.result_population(result),
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
        scope: ProducerScope,
        node: crate::domain::executable_program::types::RegionId,
    ) {
        if let Some((population, backing)) = self
            .producer_populations
            .borrow()
            .get(&(scope, node.0))
            .copied()
        {
            let population = PopulationId(population);
            self.derived.release(population);
            if backing != population {
                self.derived.release(backing);
            }
        }
    }
}

use ryo_values::domain::rows::types::Table;

pub(in crate::application::file_dag) fn same_admitted_rows(
    left: BorrowedRows<'_>,
    right: BorrowedRows<'_>,
) -> bool {
    identity::same(left, right)
}
