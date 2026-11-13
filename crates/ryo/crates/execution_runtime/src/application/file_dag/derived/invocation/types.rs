//! The data one invocation holds: the resources it prepared and the acquired handles it binds.
use super::index::service::IndexResource;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operator::contract::views::types::NativeJoin;
use ryo_execution_domain::domain::table_groups::types::Group;
use ryo_execution_store::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::LiteralText;

/// Resources are prepared once, after the scheduler's static/empty/first-row gates.
/// Regex resources are acquired separately after the checked physical/text row gates.
pub(crate) struct InvocationResources<'f> {
    pub(in crate::application::file_dag::derived) cached_graph: Option<ryo_execution_store::application::file_dag::derived::owner::cached_graph::service::CachedGraph>,
    pub(in crate::application::file_dag::derived) inputs: InvocationInputs<'f>,
}

/// The invocation binds these acquired handles once. Primitives see only the
/// immutable reader implemented for this value, never the owner or scheduler.
pub(crate) struct InvocationInputs<'f> {
    pub(in crate::application::file_dag::derived) index: Option<IndexResource<'f>>,
    pub(in crate::application::file_dag::derived) appended: Option<Vec<Value>>,
    pub(in crate::application::file_dag::derived) rank_layers: Option<Vec<Vec<LiteralText>>>,
    pub(in crate::application::file_dag::derived) native_join: Option<NativeJoin<'f>>,
    pub(in crate::application::file_dag::derived) native_storage: Option<AcquiredView>,
    pub(in crate::application::file_dag::derived) cycles: Option<AcquiredView>,
    pub(in crate::application::file_dag::derived) closure: AcquiredPartitions,
    pub(in crate::application::file_dag::derived) partitioned_closure: Option<
        ryo_execution_store::application::file_dag::closure_partitions::service::PartitionedClosure<
            'f,
        >,
    >,
    pub(in crate::application::file_dag::derived) ordered: Option<AcquiredView>,
    pub(in crate::application::file_dag::derived) dfs: Option<AcquiredView>,
    pub(in crate::application::file_dag::derived) axis: Option<
        crate::application::file_dag::derived::axis_resources::view::service::AxisResource<'f>,
    >,
    pub(in crate::application::file_dag::derived) groups: Option<Vec<Group>>,
    /// The file's profile, the observer the lent indexes report their probes to.
    #[cfg(feature = "diagnostic-row-progress")]
    pub(in crate::application::file_dag::derived) progress:
        ryo_execution_store::application::file_dag::route_counts::profile::service::RouteProfile,
}

/// What one evaluation of an invocation reads: the cached graph's answer for its row, when the invocation holds a graph,
/// and the resources its primitives read.
pub(crate) struct LentRow<'a> {
    pub(crate) cached: Option<
        ryo_execution_domain::domain::operator::contract::graph_view::types::CachedGraphRow<'a>,
    >,
    pub(crate) resources:
        &'a dyn ryo_execution_domain::domain::operator::contract::types::PrimitiveResources,
}

/// The closure partitions an invocation acquired, in partition order: the views the store leased, held as one
/// list so the engine can lend them to the domain's partition access (a foreign list cannot implement it).
#[derive(Debug, Default)]
pub(crate) struct AcquiredPartitions(pub(crate) Vec<AcquiredView>);

impl ryo_execution_domain::domain::operator::contract::views::types::ClosureParts
    for AcquiredPartitions
{
    fn len(
        &self,
    ) -> ryo_execution_domain::domain::operator::contract::views::types::PartitionCount {
        ryo_execution_domain::domain::operator::contract::views::types::PartitionCount(self.0.len())
    }
    fn get(
        &self,
        at: ryo_execution_domain::domain::operator::contract::views::types::PartitionIndex,
    ) -> Result<&ryo_derived::domain::closure_adjacency::types::ClosurePartition, OperatingError>
    {
        let view = self
            .0
            .as_slice()
            .get(at.0)
            .ok_or(OperatingError::ShapeMismatch)?
            .view()?;
        match view {
            ryo_derived::domain::contract::types::DerivedView::ClosureAdjacency(part) => Ok(part),
            _ => Err(OperatingError::WrongOperation),
        }
    }
}
