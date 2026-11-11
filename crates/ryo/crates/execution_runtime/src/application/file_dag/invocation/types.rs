//! The types of the `invocation` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use super::index::service::IndexResource;
use super::prepared_use::service::prepared;
use super::service::AcquiredPartitions;
use ryo_derived::domain::cycle_adjacency::types::CycleGraph;
use ryo_derived::domain::dfs::types::PreparedDfs;
use ryo_derived::domain::ordered::types::OrderedMatching;
use ryo_execution_domain::domain::operator::contract::{
    resources::ResourceProgram, types::OperatorContract, views::NativeJoin,
};
use ryo_execution_domain::domain::table_groups::types::Group;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_execution_domain::domain::table_reach::borrowed::IndexedRows;
use ryo_execution_domain::domain::{
    operating::errors::OperatingError, operator::types::OperatorContext,
};
use ryo_execution_store::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::{InputNumber, LiteralText};

/// The invocation binds these acquired handles once. Primitives see only the
/// immutable reader implemented for this value, never the owner or scheduler.
pub(crate) struct InvocationInputs<'f> {
    pub(crate) index: Option<IndexResource<'f>>,
    pub(crate) appended: Option<Vec<Value>>,
    pub(crate) rank_layers: Option<Vec<Vec<LiteralText>>>,
    pub(crate) native_join: Option<NativeJoin<'f>>,
    pub(crate) native_storage: Option<AcquiredView>,
    pub(crate) cycles: Option<AcquiredView>,
    pub(crate) closure: AcquiredPartitions,
    pub(crate) partitioned_closure: Option<
        ryo_execution_store::application::file_dag::closure_partitions::service::PartitionedClosure<
            'f,
        >,
    >,
    pub(crate) ordered: Option<AcquiredView>,
    pub(crate) dfs: Option<AcquiredView>,
    pub(crate) axis: Option<super::super::axis_resources::view::service::AxisResource<'f>>,
    pub(crate) groups: Option<Vec<Group>>,
    /// The file's profile, the observer the lent indexes report their probes to.
    #[cfg(feature = "diagnostic-row-progress")]
    pub(crate) progress:
        ryo_execution_store::application::file_dag::route_counts::profile::service::RouteProfile,
}
