//! Publication bodies and records contain data, never mutable owner workflows.
use crate::application::file_dag::derived::population::types::PopulationId;
use crate::application::file_dag::derived::ledger::types::BackingAddress;
use crate::domain::executable_program::types::RegionId;
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::{
    collections::representations::types::{PhysicalShape, RepresentationKind},
    types::{PublishedRelation, StoredSelection},
};
use ryo_relations::domain::relations::types::{CellKind, Relation};
use std::sync::{Arc, OnceLock};
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(in crate::application::file_dag::relations) struct SourceDescriptor {
    pub(in crate::application::file_dag::relations) relation: Relation,
    pub(in crate::application::file_dag::relations) kind: RepresentationKind,
    pub(in crate::application::file_dag::relations) kinds: Vec<CellKind>,
    pub(in crate::application::file_dag::relations) rows: RowCount,
    pub(in crate::application::file_dag::relations) storage: Vec<StorageSpan>,
    pub(in crate::application::file_dag::relations) projection:
        Vec<ryo_values::domain::plan_parts::types::ColumnIndex>,
    pub(in crate::application::file_dag::relations) source_kinds: Vec<CellKind>,
}
#[derive(Debug)]
pub(in crate::application::file_dag::relations) enum RelationBody {
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
/// Elements one borrowed storage block holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in crate::application::file_dag) struct SpanLength(pub usize);

/// One borrowed storage block: where it starts and how long it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in crate::application::file_dag) struct StorageSpan {
    pub address: BackingAddress,
    pub length: SpanLength,
}

/// One canonical DAG producer inside one invocation namespace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in crate::application::file_dag) struct ProducerKey {
    pub scope: ProducerScope,
    pub region: RegionId,
}

/// The identities one producer keeps across rules and safe recomputation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag) struct ProducerPopulation {
    pub population: PopulationId,
    pub backing: PopulationId,
}

/// A typed invocation namespace; publications retain their actual population after it ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in crate::application::file_dag) enum ProducerScope {
    File,
    Project(PopulationId),
}

/// Only this application module can construct or mutate proof state.
#[derive(Debug)]
pub(crate) struct RelationResult {
    pub(in crate::application::file_dag::relations) population: PopulationId,
    pub(in crate::application::file_dag::relations) producer: OnceLock<PopulationId>,
    pub(in crate::application::file_dag::relations) producer_scope: OnceLock<ProducerScope>,
    pub(in crate::application::file_dag::relations) generation: Arc<()>,
    pub(in crate::application::file_dag::relations) body: RelationBody,
    pub(in crate::application::file_dag::relations) inspected:
        OnceLock<Result<PhysicalShape, OperatingError>>,
    pub(in crate::application::file_dag::relations) normalized: OnceLock<Arc<Table>>,
}
impl PublishedRelation for RelationResult {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

use ryo_values::domain::rows::types::{RowCount, Table};
