//! Describe immutable allocation identity once when publishing, never by hashing rows.
use super::*;
use crate::application::file_dag::derived::ledger::Backing;
impl RelationOwner {
    fn backings(&self, record: &Arc<RelationResult>) -> Vec<Backing> {
        let mut output = Vec::new();
        Self::record_backings(record, &mut output);
        output
    }
    fn record_backings(record: &Arc<RelationResult>, output: &mut Vec<Backing>) {
        let mut metadata = size_of::<RelationResult>() + 2 * size_of::<usize>();
        match &record.body {
            RelationBody::Source(source) => {
                metadata += source.storage.capacity() * size_of::<(usize, usize)>()
                    + source.kinds.capacity() * size_of::<CellKind>()
                    + source.projection.capacity()
                        * size_of::<ryo_values::domain::plan_parts::types::ColumnIndex>()
                    + source.source_kinds.capacity() * size_of::<CellKind>();
            }
            RelationBody::Computed(table) => output.push(Self::table_backing(table)),
            RelationBody::Compact(table) => {
                let words = crate::domain::table_words::service::compact_backing(table);
                output.push(Backing {
                    key: (22, Arc::as_ptr(words) as usize),
                    retention_units: 0,
                    bytes: size_of::<ryo_values::domain::stored_facts::types::KeptWords>()
                        + 2 * size_of::<usize>()
                        + words.0.0.capacity(),
                });
                output.push(Backing {
                    key: (24, Arc::as_ptr(table) as usize),
                    retention_units: 0,
                    bytes: crate::domain::table_words::service::compact_metadata_bytes(table),
                });
            }
            RelationBody::Selected { source, selection } => {
                Self::record_backings(source, output);
                output.push(Backing {
                    key: (6, Arc::as_ptr(selection) as usize),
                    retention_units: 0,
                    bytes: size_of::<StoredSelection>()
                        + 2 * size_of::<usize>()
                        + selection.columns.capacity()
                            * size_of::<ryo_values::domain::plan_parts::types::ColumnIndex>()
                        + selection.kinds.capacity() * size_of::<CellKind>(),
                });
                output.push(Self::rows_backing(&selection.rows));
            }
        }
        output.push(Backing {
            key: (7, Arc::as_ptr(record) as usize),
            retention_units: 0,
            bytes: metadata,
        });
        if let Some(table) = record.normalized.get() {
            output.push(Self::table_backing(table));
        }
    }
    pub(super) fn rows_backing(rows: &Arc<Vec<u32>>) -> Backing {
        Backing {
            key: (1, Arc::as_ptr(rows) as usize),
            retention_units: rows.capacity(),
            bytes: size_of::<Vec<u32>>()
                + 2 * size_of::<usize>()
                + rows.capacity() * size_of::<u32>(),
        }
    }
    pub(super) fn table_backing(table: &Arc<Table>) -> Backing {
        Backing {
            key: (0, Arc::as_ptr(table) as usize),
            retention_units: 0,
            bytes: table_bytes(table),
        }
    }
    fn resource_backings(
        resources: &Option<Arc<[super::super::derived::view::ResourceDependency]>>,
        output: &mut Vec<Backing>,
    ) {
        let Some(resources) = resources else {
            return;
        };
        output.push(Backing {
            key: (17, resources.as_ptr() as usize),
            retention_units: 0,
            bytes: 2 * size_of::<usize>() + std::mem::size_of_val(resources.as_ref()),
        });
        for resource in resources.iter() {
            output.extend(resource.backings.iter().copied());
        }
    }
    pub(super) fn publication(
        &self,
        record: Arc<RelationResult>,
    ) -> Result<PublishedInput, OperatingError> {
        self.leased_publication(record, None)
    }
    pub(super) fn publication_with_resources(
        &self,
        record: Arc<RelationResult>,
        resources: [Option<super::super::derived::view::AcquiredView>; 2],
    ) -> Result<PublishedInput, OperatingError> {
        let dependencies = resources
            .iter()
            .flatten()
            .map(|view| view.dependency())
            .collect::<Vec<_>>();
        let dependencies = (!dependencies.is_empty()).then(|| Arc::from(dependencies));
        self.leased_publication(record, dependencies)
    }
    fn leased_publication(
        &self,
        record: Arc<RelationResult>,
        resources: Option<Arc<[super::super::derived::view::ResourceDependency]>>,
    ) -> Result<PublishedInput, OperatingError> {
        let mut backings = self.backings(&record);
        Self::resource_backings(&resources, &mut backings);
        let lease = self.derived.required_lease(&backings)?;
        Ok(PublishedInput::new(record, lease, resources))
    }
    pub(in crate::application::file_dag) fn required_publication(
        &self,
        retained: &RetainedPublication,
    ) -> Result<PublishedInput, OperatingError> {
        self.leased_publication(Arc::clone(retained.record()), retained.resources.clone())
    }
    fn finalized_backings(record: &RelationResult) -> bool {
        match &record.body {
            RelationBody::Computed(_) | RelationBody::Compact(_) => true,
            RelationBody::Selected { source, .. } => Self::finalized_backings(source),
            RelationBody::Source(_) => {
                record.normalized.get().is_some()
                    || matches!(record.inspected.get(), Some(Ok(PhysicalShape::Complete)))
            }
        }
    }
    pub(in crate::application::file_dag) fn retain_publication(
        &self,
        input: &PublishedInput,
        limit: usize,
    ) -> Result<Option<RetainedPublication>, OperatingError> {
        let record = Arc::clone(input.record());
        // Optional records must have an immutable backing set. Source shape is
        // still lazy: declining retention must not inspect or normalize it.
        if !Self::finalized_backings(&record) {
            return Ok(None);
        }
        let mut backings = self.backings(&record);
        Self::resource_backings(&input.resources, &mut backings);
        let Some(lease) = self.derived.node_optional_lease(&backings, limit)? else {
            return Ok(None);
        };
        Ok(Some(RetainedPublication::new(
            record,
            lease,
            input.resources.clone(),
        )))
    }
}

pub(in crate::application::file_dag) fn bytes(result: &RelationResult) -> usize {
    let owned = match &result.body {
        RelationBody::Computed(table) => table_bytes(table),
        RelationBody::Compact(table) => {
            crate::domain::table_words::service::compact_metadata_bytes(table)
                + crate::domain::table_words::service::compact_backing(table)
                    .0
                    .0
                    .capacity()
        }
        RelationBody::Source(source) => {
            source.storage.capacity() * size_of::<(usize, usize)>()
                + source.kinds.capacity()
                    * size_of::<ryo_relations::domain::relations::types::CellKind>()
                + source.projection.capacity()
                    * size_of::<ryo_values::domain::plan_parts::types::ColumnIndex>()
                + source.source_kinds.capacity()
                    * size_of::<ryo_relations::domain::relations::types::CellKind>()
        }
        RelationBody::Selected { selection, .. } => {
            selection.rows.capacity() * size_of::<u32>()
                + selection.columns.capacity()
                    * size_of::<ryo_values::domain::plan_parts::types::ColumnIndex>()
                + selection.kinds.capacity()
                    * size_of::<ryo_relations::domain::relations::types::CellKind>()
                + size_of::<StoredSelection>()
        }
    };
    owned
        + result
            .normalized
            .get()
            .map_or(0, |table| table_bytes(table))
        + size_of::<RelationResult>()
        + 2 * size_of::<usize>()
}

fn table_bytes(table: &ryo_values::domain::rows::types::Table) -> usize {
    table
        .cells
        .capacity()
        .saturating_mul(size_of::<ryo_relations::domain::relations::types::Value>())
        .saturating_add(table.kinds.capacity().saturating_mul(size_of::<CellKind>()))
        .saturating_add(
            size_of::<ryo_values::domain::rows::types::Table>() + 2 * size_of::<usize>(),
        )
}
