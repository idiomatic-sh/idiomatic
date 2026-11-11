//! Publication bodies and records contain data, never mutable owner workflows.
use crate::application::file_dag::derived::ledger::types::BackingAddress;
use crate::application::file_dag::derived::population::service::PopulationId;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::table_model::{
    types::PublishedRelation,
    types::{PhysicalShape, RepresentationKind},
};
use ryo_relations::domain::relations::types::{CellKind, Relation};
use std::sync::{Arc, OnceLock};
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct SourceDescriptor {
    pub(crate) relation: Relation,
    pub(crate) kind: RepresentationKind,
    pub(crate) kinds: Vec<CellKind>,
    pub(crate) rows: RowCount,
    pub(crate) storage: Vec<StorageSpan>,
    pub(crate) projection: Vec<ryo_values::domain::plan_parts::types::ColumnIndex>,
    pub(crate) source_kinds: Vec<CellKind>,
}
#[derive(Debug)]
pub(crate) enum RelationBody {
    Source(SourceDescriptor),
    Computed(Arc<Table>),
    Compact(Arc<ryo_values::domain::table_words::types::CompactTable>),
}
/// Elements one borrowed storage block holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct SpanLength(pub usize);

/// One borrowed storage block: where it starts and how long it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct StorageSpan {
    pub address: BackingAddress,
    pub length: SpanLength,
}

/// One canonical DAG producer inside one invocation namespace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ProducerKey {
    pub scope: ProducerScope,
    pub producer: crate::application::file_dag::relations::types::ProducerId,
}

/// The identities one producer keeps across rules and safe recomputation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ProducerPopulation {
    pub population: PopulationId,
    pub backing: PopulationId,
}

/// A typed invocation namespace; publications retain their actual population after it ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProducerScope {
    /// A file-scoped publication.
    File,
    /// A project-scoped publication of one population.
    Project(PopulationId),
}

/// Only this application module can construct or mutate proof state.
#[derive(Debug)]
pub(crate) struct RelationResult {
    pub(crate) population: PopulationId,
    pub(crate) producer: OnceLock<PopulationId>,
    pub(crate) producer_scope: OnceLock<ProducerScope>,
    pub(crate) generation:
        Arc<crate::application::file_dag::relations::generation::service::GenerationLease>,
    pub(crate) body: RelationBody,
    pub(crate) inspected: OnceLock<Result<PhysicalShape, OperatingError>>,
    pub(crate) normalized: OnceLock<Arc<Table>>,
}
impl PublishedRelation for RelationResult {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

use ryo_values::domain::rows::types::{RowCount, Table};

/// What one publication publishes: a source's rows, a supplied compact or flat source, a computed table, or packed words with the
/// arenas they reference.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::application::file_dag::relations::publication::types::PublicationBody;
/// use ryo_execution_store::application::file_dag::fixtures::program_derived_port;
/// use ryo_execution_store::application::file_dag::relations::{interface::RelationsPort, service::RelationOwner};
/// use ryo_execution_store::application::file_dag::relations::types::ReuseEligibility;
/// use ryo_relations::domain::relations::types::{CellKind, FileId, Value};
/// use ryo_values::domain::rows::types::{RowCount, Table};
/// let table = Table { kinds: vec![CellKind::File], cells: vec![Value::File(FileId(0))], count: RowCount(1) };
/// let owner = RelationOwner::fixture(ReuseEligibility::Eligible, program_derived_port());
/// assert!(owner.publish(PublicationBody::Computed(table)).is_ok());
/// ```
pub enum PublicationBody<'a> {
    /// The rows of a source relation.
    Source {
        relation: Relation,
        rows: &'a ryo_execution_domain::domain::table_model::types::Rows<'a>,
    },
    /// A supplied compact source.
    Compact(Arc<ryo_values::domain::table_words::types::CompactTable>),
    /// A supplied flat source, its exact geometry kept for first-borrow inspection.
    Shared(Arc<Table>),
    /// A computed table.
    Computed(Table),
    /// Encoded words and every existing arena they reference.
    Packed {
        words: ryo_values::domain::stored_facts::types::KeptWords,
        arenas: &'a [&'a ryo_values::domain::stored_facts::types::ArenaBytes],
    },
}
