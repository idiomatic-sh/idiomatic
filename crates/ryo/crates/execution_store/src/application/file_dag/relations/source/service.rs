//! Source custody: publishing supplied inputs as owner-issued publications.
use crate::application::file_dag::relations::*;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
impl RelationOwner {
    /// Publishes the rows of a source relation.
    pub fn source(
        &self,
        relation: Relation,
        rows: &Rows<'_>,
    ) -> Result<PublishedInput, OperatingError> {
        self.source_borrowed(relation, RepresentationInput::from(rows))
    }
    pub(in crate::application::file_dag::relations) fn source_borrowed(
        &self,
        relation: Relation,
        raw: RepresentationInput<'_>,
    ) -> Result<PublishedInput, OperatingError> {
        let population = match raw {
            other => other,
        };
        if let Some(found) = self.sources.borrow().get(&relation) {
            if identity::matches(&found.population, population) == Sameness::Different {
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
        if self.stable == ReuseEligibility::Eligible && self.views.get() < self.max_views {
            let mut sources = self.sources.borrow_mut();
            let entry = match sources.entry(relation) {
                std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
                std::collections::hash_map::Entry::Vacant(entry) => entry.insert(SourceViews {
                    population: identity::describe(relation, population),
                    // Graph aliases share a separate admitted ID. Pinning a
                    // graph source must not promote unrelated ordinary views
                    // keyed by the raw relation's primary population.
                    canonical_population: self.allocate_population()?,
                    views: FxHashMap::default(),
                }),
            };
            entry
                .views
                .insert(identity::describe(relation, raw), result.clone());
            self.views.set(crate::application::file_dag::relations::types::PinnedViewCount(self.views.get().0 + 1));
        }
        Ok(result)
    }
    /// Custody of a supplied compact source; no kernel permit is issued here.
    pub fn source_compact(
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
    pub fn source_shared_computed(
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
    /// Publishes a computed table.
    pub fn computed(&self, table: Table) -> Result<PublishedInput, OperatingError> {
        let input = RepresentationInput::Computed(&table);
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
}
