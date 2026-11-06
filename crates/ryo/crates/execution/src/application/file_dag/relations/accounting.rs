//! Describe immutable allocation identity once when publishing, never by hashing rows.
use super::*;
use crate::application::file_dag::derived::ledger::types::{
    Backing, BackingAddress, BackingKey, BackingTag, ByteCount, UnitCount,
};
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
            RelationBody::Positioned {
                source,
                positions,
                shape,
                kinds,
            } => {
                Self::record_backings(source, output);
                output.push(Backing {
                    key: BackingKey {
                        tag: BackingTag(25),
                        address: BackingAddress(Arc::as_ptr(positions) as usize),
                    },
                    retention_units: UnitCount(positions.slots.0.capacity()),
                    bytes: ByteCount(
                        size_of::<crate::domain::positions::types::Positions>()
                            + 2 * size_of::<usize>()
                            + positions.slots.0.capacity() * size_of::<u32>(),
                    ),
                });
                output.push(Backing {
                    key: BackingKey {
                        tag: BackingTag(26),
                        address: BackingAddress(Arc::as_ptr(shape) as usize),
                    },
                    retention_units: UnitCount(0),
                    bytes: ByteCount(
                        size_of::<crate::domain::lowering::types::LoweredShape>()
                            + 2 * size_of::<usize>()
                            + shape.columns.capacity()
                                * size_of::<crate::domain::lowering::types::LoweredColumn>(),
                    ),
                });
                output.push(Backing {
                    key: BackingKey {
                        tag: BackingTag(27),
                        address: BackingAddress(kinds.as_ptr() as usize),
                    },
                    retention_units: UnitCount(0),
                    bytes: ByteCount(
                        std::mem::size_of_val(kinds.as_ref()) + 2 * size_of::<usize>(),
                    ),
                });
            }
            RelationBody::Computed(table) => output.push(Self::table_backing(table)),
            RelationBody::Compact(table) => {
                let words = ryo_values::domain::table_words::service::compact_backing(table);
                output.push(Backing {
                    key: BackingKey {
                        tag: BackingTag(22),
                        address: BackingAddress(Arc::as_ptr(words) as usize),
                    },
                    retention_units: UnitCount(0),
                    bytes: ByteCount(
                        size_of::<ryo_values::domain::stored_facts::types::KeptWords>()
                            + 2 * size_of::<usize>()
                            + words.0.0.capacity(),
                    ),
                });
                output.push(Backing {
                    key: BackingKey {
                        tag: BackingTag(24),
                        address: BackingAddress(Arc::as_ptr(table) as usize),
                    },
                    retention_units: UnitCount(0),
                    bytes: ByteCount(
                        ryo_values::domain::table_words::service::compact_metadata_bytes(table).0,
                    ),
                });
            }
            RelationBody::Selected { source, selection } => {
                Self::record_backings(source, output);
                output.push(Backing {
                    key: BackingKey {
                        tag: BackingTag(6),
                        address: BackingAddress(Arc::as_ptr(selection) as usize),
                    },
                    retention_units: UnitCount(0),
                    bytes: ByteCount(
                        size_of::<StoredSelection>()
                            + 2 * size_of::<usize>()
                            + selection.columns.capacity()
                                * size_of::<ryo_values::domain::plan_parts::types::ColumnIndex>()
                            + selection.kinds.capacity() * size_of::<CellKind>(),
                    ),
                });
                output.push(Self::rows_backing(&selection.rows));
            }
        }
        output.push(Backing {
            key: BackingKey {
                tag: BackingTag(7),
                address: BackingAddress(Arc::as_ptr(record) as usize),
            },
            retention_units: UnitCount(0),
            bytes: ByteCount(metadata),
        });
        if let Some(table) = record.normalized.get() {
            output.push(Self::table_backing(table));
        }
    }
    pub(super) fn rows_backing(rows: &Arc<Vec<u32>>) -> Backing {
        Backing {
            key: BackingKey {
                tag: BackingTag(1),
                address: BackingAddress(Arc::as_ptr(rows) as usize),
            },
            retention_units: UnitCount(rows.capacity()),
            bytes: crate::application::file_dag::derived::ledger::capacity::shared_vector(rows),
        }
    }
    pub(super) fn table_backing(table: &Arc<Table>) -> Backing {
        Backing {
            key: BackingKey {
                tag: BackingTag(0),
                address: BackingAddress(Arc::as_ptr(table) as usize),
            },
            retention_units: UnitCount(0),
            bytes: ByteCount(table_bytes(table)),
        }
    }
    fn resource_backings(
        resources: &Option<
            Arc<[super::super::derived::owner::acquisition::admission::service::ResourceDependency]>,
        >,
        output: &mut Vec<Backing>,
    ) {
        let Some(resources) = resources else {
            return;
        };
        output.push(Backing {
            key: BackingKey {
                tag: BackingTag(17),
                address: BackingAddress(resources.as_ptr() as usize),
            },
            retention_units: UnitCount(0),
            bytes: ByteCount(2 * size_of::<usize>() + std::mem::size_of_val(resources.as_ref())),
        });
        for resource in resources.iter() {
            output.extend(resource.backings().iter().copied());
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
        resources: [Option<super::super::derived::owner::acquisition::admission::service::AcquiredView>;
            2],
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
        resources: Option<
            Arc<[super::super::derived::owner::acquisition::admission::service::ResourceDependency]>,
        >,
    ) -> Result<PublishedInput, OperatingError> {
        let mut backings = self.backings(&record);
        Self::resource_backings(&resources, &mut backings);
        let lease = self.derived.required_lease(&backings)?;
        Ok(PublishedInput::new(record, lease, resources))
    }
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
