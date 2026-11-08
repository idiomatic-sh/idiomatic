//! Publication bodies and records contain data, never mutable owner workflows.
use crate::application::file_dag::derived::population::types::PopulationId;
use crate::application::file_dag::derived::ledger::types::BackingAddress;
use ryo_execution_domain::domain::executable_program::types::RegionId;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::table_model::{
    types::{PhysicalShape, RepresentationKind},
    types::PublishedRelation,
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
    pub(crate) projection:
        Vec<ryo_values::domain::plan_parts::types::ColumnIndex>,
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
    pub region: RegionId,
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
    File,
    Project(PopulationId),
}

/// Only this application module can construct or mutate proof state.
#[derive(Debug)]
pub struct RelationResult {
    pub(crate) population: PopulationId,
    pub(crate) producer: OnceLock<PopulationId>,
    pub(crate) producer_scope: OnceLock<ProducerScope>,
    pub(crate) generation: Arc<crate::application::file_dag::relations::generation::service::GenerationLease>,
    pub(crate) body: RelationBody,
    pub(crate) inspected:
        OnceLock<Result<PhysicalShape, OperatingError>>,
    pub(crate) normalized: OnceLock<Arc<Table>>,
}
impl PublishedRelation for RelationResult {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

use ryo_values::domain::rows::types::{RowCount, Table};
