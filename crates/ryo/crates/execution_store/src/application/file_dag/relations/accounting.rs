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
            RelationBody::Computed(table) => output.push(Self::table_backing(table)),
            RelationBody::Compact(table) => {
                let words = ryo_values::domain::compact_table::service::compact_backing(table);
                output.push(Backing {
                    key: BackingKey {
                        tag: BackingTag(22),
                        address: BackingAddress(Arc::as_ptr(&words.0).addr()),
                    },
                    retention_units: UnitCount(0),
                    bytes: ByteCount(
                        size_of::<ryo_values::domain::stored_facts::types::KeptWords>()
                            + 2 * size_of::<usize>()
                            + words.0.0.0.capacity(),
                    ),
                });
                output.push(Backing {
                    key: BackingKey {
                        tag: BackingTag(24),
                        address: BackingAddress(Arc::as_ptr(table).addr()),
                    },
                    retention_units: UnitCount(0),
                    bytes: crate::application::file_dag::derived::ledger::capacity::compact_table(
                        table,
                    ),
                });
            }
        }
        output.push(Backing {
            key: BackingKey {
                tag: BackingTag(7),
                address: BackingAddress(Arc::as_ptr(record).addr()),
            },
            retention_units: UnitCount(0),
            bytes: ByteCount(metadata),
        });
        if let Some(table) = record.normalized.get() {
            output.push(Self::table_backing(table));
        }
    }
    pub(super) fn table_backing(table: &Arc<Table>) -> Backing {
        Backing {
            key: BackingKey {
                tag: BackingTag(0),
                address: BackingAddress(Arc::as_ptr(table).addr()),
            },
            retention_units: UnitCount(0),
            bytes: table_bytes(table),
        }
    }
    /// Publishes `record` with its backing accounted to the ledger.
    pub(crate) fn publication(
        &self,
        record: Arc<RelationResult>,
    ) -> Result<PublishedInput, OperatingError> {
        let backings = self.backings(&record);
        let lease = self.derived.required_lease(&backings)?;
        Ok(PublishedInput::new(record, lease))
    }
}

fn table_bytes(table: &ryo_values::domain::rows::types::Table) -> ByteCount {
    ByteCount(
        table
            .cells
            .capacity()
            .saturating_mul(size_of::<ryo_relations::domain::relations::types::Value>())
            .saturating_add(table.kinds.capacity().saturating_mul(size_of::<CellKind>()))
            .saturating_add(
                size_of::<ryo_values::domain::rows::types::Table>() + 2 * size_of::<usize>(),
            ),
    )
}
