//! Every production file outside the owner that names a leased view or handle. Each reads
//! only through accessors; the authority it holds cannot be forged (`entry::Authority::control`).
use super::entry::Authority;

macro_rules! file {
    ($path:literal) => {
        concat!("application/file_dag/", $path)
    };
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Holder {
    AxisResources,
    AxisView,
    ClosurePartitions,
    ExecutionLease,
    Invocation,
    InvocationIndex,
    InvocationNative,
    InvocationViews,
    RelationAccounting,
    RelationHandle,
    SelectedAccess,
    RelationSelection,
    Selected,
    Workspace,
    GraphView,
    DerivedAccessPrepare,
    DerivedAccessRequest,
    DerivedAccessViews,
    DerivedAccessRelations,
    DerivedAccessScope,
    DerivedAccessServices,
    DerivedAccessPreparation,
    DerivedAccessOrders,
}
impl Holder {
    pub(super) const ALL: [Holder; 23] = [
        Holder::AxisResources,
        Holder::AxisView,
        Holder::ClosurePartitions,
        Holder::ExecutionLease,
        Holder::Invocation,
        Holder::InvocationIndex,
        Holder::InvocationNative,
        Holder::InvocationViews,
        Holder::RelationAccounting,
        Holder::RelationHandle,
        Holder::SelectedAccess,
        Holder::RelationSelection,
        Holder::Selected,
        Holder::Workspace,
        Holder::GraphView,
        Holder::DerivedAccessPrepare,
        Holder::DerivedAccessRequest,
        Holder::DerivedAccessViews,
        Holder::DerivedAccessRelations,
        Holder::DerivedAccessScope,
        Holder::DerivedAccessServices,
        Holder::DerivedAccessPreparation,
        Holder::DerivedAccessOrders,
    ];
    pub(super) fn path(self) -> &'static str {
        match self {
            Holder::AxisResources => file!("axis_resources.rs"),
            Holder::AxisView => file!("axis_resources/view.rs"),
            Holder::ClosurePartitions => file!("closure_partitions.rs"),
            Holder::ExecutionLease => file!("execution/lease.rs"),
            Holder::Invocation => file!("invocation.rs"),
            Holder::InvocationIndex => file!("invocation/index.rs"),
            Holder::InvocationNative => file!("invocation/native/service.rs"),
            Holder::InvocationViews => file!("invocation/views.rs"),
            Holder::RelationAccounting => file!("relations/accounting.rs"),
            Holder::RelationHandle => file!("relations/handle/service.rs"),
            Holder::SelectedAccess => file!("relations/selected_access.rs"),
            Holder::RelationSelection => file!("relations/selection.rs"),
            Holder::Selected => file!("selected.rs"),
            Holder::Workspace => file!("workspace.rs"),
            Holder::GraphView => "domain/operator/contract/graph_view.rs",
            Holder::DerivedAccessPrepare => file!("derived/prepare.rs"),
            Holder::DerivedAccessRequest => file!("request.rs"),
            Holder::DerivedAccessViews => file!("derived/request.rs"),
            Holder::DerivedAccessRelations => file!("relations.rs"),
            Holder::DerivedAccessScope => file!("scope.rs"),
            Holder::DerivedAccessServices => file!("program/facade_runtime/types.rs"),
            Holder::DerivedAccessPreparation => file!("program/preparation/service.rs"),
            Holder::DerivedAccessOrders => file!("program/preparation/service/orders/service.rs"),
        }
    }
    /// The authorities its source names, exhaustively.
    pub(super) fn holds(self) -> &'static [Authority] {
        match self {
            Holder::AxisResources
            | Holder::AxisView
            | Holder::ClosurePartitions
            | Holder::InvocationIndex
            | Holder::InvocationNative
            | Holder::InvocationViews
            | Holder::SelectedAccess
            | Holder::RelationSelection
            | Holder::Selected
            | Holder::Workspace => &[Authority::AcquiredView],
            Holder::ExecutionLease | Holder::GraphView => &[Authority::CachedGraph],
            Holder::Invocation => &[Authority::AcquiredView, Authority::CachedGraph],
            Holder::RelationAccounting => &[Authority::AcquiredView, Authority::ResourceDependency],
            Holder::RelationHandle => &[Authority::ResourceDependency],
            Holder::DerivedAccessPrepare
            | Holder::DerivedAccessRequest
            | Holder::DerivedAccessViews
            | Holder::DerivedAccessRelations
            | Holder::DerivedAccessScope
            | Holder::DerivedAccessServices
            | Holder::DerivedAccessPreparation
            | Holder::DerivedAccessOrders => &[Authority::DerivedAccess],
        }
    }
}
