//! The data one invocation holds: the resources it prepared and the acquired handles it binds.
use super::index::service::IndexResource;
use crate::application::file_dag::derived::view::service::LeasedView;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operator::contract::views::types::NativeJoin;
use ryo_execution_domain::domain::table_groups::types::Group;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::LiteralText;

/// Resources are prepared once, after the scheduler's static/empty/first-row gates.
/// Regex resources are acquired separately after the checked physical/text row gates.
pub(crate) struct InvocationResources<'f> {
    pub(crate) cached_graph:
        Option<crate::application::file_dag::derived::graph::service::LeasedGraph>,
    pub(crate) inputs: InvocationInputs<'f>,
}

/// The invocation binds these acquired handles once. Primitives see only the
/// immutable reader implemented for this value, never the owner or scheduler.
pub(crate) struct InvocationInputs<'f> {
    pub(crate) index: Option<IndexResource<'f>>,
    pub(crate) appended: Option<Vec<Value>>,
    pub(crate) rank_layers: Option<Vec<Vec<LiteralText>>>,
    pub(crate) native_join: Option<NativeJoin<'f>>,
    pub(crate) native_storage: Option<LeasedView>,
    pub(crate) cycles: Option<LeasedView>,
    pub(crate) closure: AcquiredPartitions,
    pub(crate) partitioned_closure: Option<
        ryo_execution_store::application::file_dag::closure_partitions::service::PartitionedClosure<
            'f,
        >,
    >,
    pub(crate) ordered: Option<LeasedView>,
    pub(crate) dfs: Option<LeasedView>,
    pub(crate) axis:
        Option<crate::application::file_dag::axis_resources::view::service::AxisResource<'f>>,
    pub(crate) groups: Option<Vec<Group>>,
    /// The file's profile, the observer the lent indexes report their probes to.
    #[cfg(feature = "diagnostic-row-progress")]
    pub(crate) progress:
        ryo_execution_store::application::file_dag::route_counts::profile::service::RouteProfile,
}

/// The closure partitions an invocation acquired, in partition order: the views the store leased, held as one
/// list so the engine can lend them to the domain's partition access (a foreign list cannot implement it).
#[derive(Debug, Default)]
pub(crate) struct AcquiredPartitions(pub(crate) Vec<LeasedView>);

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
