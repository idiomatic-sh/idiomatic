//! One file generation owns representation descriptors and lazy admission proofs.
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::views::BorrowedRows;
use crate::domain::table_model::collections::representations::{
    publication::PublishedRelation,
    service::{kind, registered},
    types::{PhysicalShape, RepresentationKind},
};
use crate::domain::table_model::{
    service::materialize,
    types::{Rows, SelectedRows, StoredSelection, Table},
};
use ryo_language::domain::relations::types::{CellKind, Relation};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    sync::{Arc, OnceLock},
};
mod identity;
mod produced;
pub(super) use produced::ProducedSelection;

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
mod record;
pub(crate) use record::RelationResult;

struct SourceViews {
    population: SourceDescriptor,
    views: HashMap<SourceDescriptor, Arc<RelationResult>>,
}

pub(crate) struct RelationOwner {
    generation: RefCell<Arc<()>>,
    stable: bool,
    sources: RefCell<HashMap<Relation, SourceViews>>,
    max_views: usize,
    views: Cell<usize>,
    pub checks: Cell<u64>,
    pub reuses: Cell<u64>,
}
impl RelationOwner {
    pub(super) fn new(stable: bool) -> Self {
        Self {
            generation: RefCell::new(Arc::new(())),
            stable,
            sources: RefCell::new(HashMap::new()),
            max_views: 1024,
            views: Cell::new(0),
            checks: Cell::new(0),
            reuses: Cell::new(0),
        }
    }
    pub(super) fn for_request(stable: bool, views: usize) -> Self {
        let mut owner = Self::new(stable);
        owner.max_views = views;
        owner
    }
    pub(super) fn source(
        &self,
        relation: Relation,
        rows: &Rows<'_>,
    ) -> Result<Arc<RelationResult>, OperatingError> {
        let raw = BorrowedRows::from(rows);
        let population = match raw {
            BorrowedRows::Selected { source, .. } => BorrowedRows::Stored(source),
            other => other,
        };
        if let Some(found) = self.sources.borrow().get(&relation) {
            if !identity::matches(&found.population, population) {
                return Err(OperatingError::NotRead);
            }
            if let Some(view) = found.views.get(&identity::describe(relation, raw)) {
                return Ok(Arc::clone(view));
            }
        }
        let result = Arc::new(RelationResult {
            generation: Arc::clone(&self.generation.borrow()),
            kind: kind(raw),
            body: RelationBody::Source(identity::describe(relation, raw)),
            inspected: OnceLock::new(),
            normalized: OnceLock::new(),
        });
        if self.stable && self.views.get() < self.max_views {
            let mut sources = self.sources.borrow_mut();
            let entry = sources.entry(relation).or_insert_with(|| SourceViews {
                population: identity::describe(relation, population),
                views: HashMap::new(),
            });
            entry
                .views
                .insert(identity::describe(relation, raw), Arc::clone(&result));
            self.views.set(self.views.get() + 1);
        }
        Ok(result)
    }
    pub(super) fn computed(&self, table: Table) -> Result<Arc<RelationResult>, OperatingError> {
        let input = BorrowedRows::Computed(&table);
        let contract = input.contract()?;
        registered(contract.kind).inspect(&contract)?;
        Ok(Arc::new(RelationResult {
            generation: Arc::clone(&self.generation.borrow()),
            kind: RepresentationKind::Computed,
            body: RelationBody::Computed(Arc::new(table)),
            inspected: OnceLock::from(Ok(PhysicalShape::Complete)),
            normalized: OnceLock::new(),
        }))
    }
    pub(super) fn publish_selection(
        &self,
        produced: ProducedSelection,
    ) -> Result<Arc<RelationResult>, OperatingError> {
        let ProducedSelection { source, selection } = produced;
        self.generation(&source)?;
        Ok(Arc::new(RelationResult {
            generation: Arc::clone(&self.generation.borrow()),
            kind: RepresentationKind::Selected,
            body: RelationBody::Selected { source, selection },
            inspected: OnceLock::from(Ok(PhysicalShape::Complete)),
            normalized: OnceLock::new(),
        }))
    }
    pub(super) fn stable(&self) -> bool {
        self.stable
    }
    pub(super) fn produce_selection<'f>(
        &self,
        program: &Arc<crate::domain::stored_selection::types::Selector>,
        read: &dyn Fn(Relation) -> Result<Rows<'f>, OperatingError>,
        context: &mut crate::domain::operating::types::OperatorContext<'_>,
    ) -> Result<Option<ProducedSelection>, OperatingError> {
        produced::select(self, program, read, context)
    }
    /// Explicitly ends the immutable resident-population lease before release or refill.
    pub(super) fn release_population(&self) {
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
    pub(super) fn matches(&self, result: &RelationResult, input: BorrowedRows<'_>) -> bool {
        self.generation(result).is_ok()&&(result.normalized.get().is_some_and(|table|matches!(input,BorrowedRows::Computed(actual)if std::ptr::eq(actual,table.as_ref())))||match &result.body{
   RelationBody::Source(source)=>identity::matches(source,input),
   RelationBody::Computed(table)=>matches!(input,BorrowedRows::Computed(actual)if std::ptr::eq(actual,table.as_ref())),
   RelationBody::Selected{selection,source}=>matches!(input,BorrowedRows::Selected{source:actual,positions,columns,kinds}if std::ptr::eq(positions,selection.rows.as_slice())&&columns==selection.columns&&kinds==selection.kinds&&self.matches(source,BorrowedRows::Stored(actual))),
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
        result: &dyn PublishedRelation,
        rows: Rows<'_>,
    ) -> Result<Option<Arc<Table>>, OperatingError> {
        let _checked = self.normalize(result, rows)?;
        Ok(self.private(result)?.normalized.get().cloned())
    }
    /// The caller invokes this only at its already declared physical demand gate.
    pub(super) fn inspect(
        &self,
        result: &dyn PublishedRelation,
        input: BorrowedRows<'_>,
    ) -> Result<PhysicalShape, OperatingError> {
        let result = self.private(result)?;
        if !self.matches(result, input) {
            return Err(OperatingError::NotRead);
        }
        if result.normalized.get().is_some_and(|table|matches!(input,BorrowedRows::Computed(actual)if std::ptr::eq(actual,table.as_ref()))){self.reuses.set(self.reuses.get()+1);return Ok(PhysicalShape::Complete)}
        if result.inspected.get().is_some() {
            self.reuses.set(self.reuses.get() + 1);
        }
        let outcome = result
            .inspected
            .get_or_init(|| {
                self.checks.set(self.checks.get() + 1);
                let contract = input.contract()?;
                registered(contract.kind).inspect(&contract)
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
        result: &'r dyn PublishedRelation,
        rows: Rows<'f>,
    ) -> Result<Rows<'r>, OperatingError>
    where
        'f: 'r,
    {
        let result = self.private(result)?;
        match self.inspect(result, BorrowedRows::from(&rows))? {
            PhysicalShape::Complete => Ok(rows),
            PhysicalShape::ReadablePhysicalFallback => {
                let table = result.normalized.get_or_init(|| {
                    let mut table = Table::default();
                    materialize(&rows, &mut table, &mut Vec::new());
                    Arc::new(table)
                });
                Ok(Rows::Computed(table))
            }
        }
    }
}
pub(super) fn bytes(result: &RelationResult) -> usize {
    let owned = match &result.body {
        RelationBody::Computed(table) => super::service::table_bytes(table),
        RelationBody::Source(source) => {
            source.storage.capacity() * size_of::<(usize, usize)>()
                + source.kinds.capacity()
                    * size_of::<ryo_language::domain::relations::types::CellKind>()
                + source.projection.capacity()
                    * size_of::<ryo_values::domain::plan_parts::types::ColumnIndex>()
                + source.source_kinds.capacity()
                    * size_of::<ryo_language::domain::relations::types::CellKind>()
        }
        RelationBody::Selected { selection, .. } => {
            selection.rows.capacity() * size_of::<u32>()
                + selection.columns.capacity()
                    * size_of::<ryo_values::domain::plan_parts::types::ColumnIndex>()
                + selection.kinds.capacity()
                    * size_of::<ryo_language::domain::relations::types::CellKind>()
                + size_of::<StoredSelection>()
        }
    };
    owned
        + result
            .normalized
            .get()
            .map_or(0, |table| super::service::table_bytes(table))
        + size_of::<RelationResult>()
        + 2 * size_of::<usize>()
}

#[cfg(test)]
mod owner_test;
