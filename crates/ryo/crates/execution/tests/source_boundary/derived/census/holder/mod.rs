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
    InvocationViews,
    Workspace,
    GraphView,
    DerivedAccessPrepare,
    DerivedAccessRequest,
    DerivedAccessViews,
    DerivedAccessRelations,
    DerivedAccessScope,
    DerivedAccessServices,
    DerivedAccessPreparation,
    DerivedAccessPreparedProgram,
    DerivedAccessPreparedViews,
    DerivedAccessPrepareUseCase,
    AddedClosurePartitionsSchemas,
    AddedWorkspaceService,
}
impl Holder {
    pub(super) const ALL: [Holder; 21] = [
        Holder::AxisResources,
        Holder::AxisView,
        Holder::ClosurePartitions,
        Holder::ExecutionLease,
        Holder::Invocation,
        Holder::InvocationIndex,
        Holder::InvocationViews,
        Holder::Workspace,
        Holder::GraphView,
        Holder::DerivedAccessPrepare,
        Holder::DerivedAccessRequest,
        Holder::DerivedAccessViews,
        Holder::DerivedAccessRelations,
        Holder::DerivedAccessScope,
        Holder::DerivedAccessServices,
        Holder::DerivedAccessPreparation,
        Holder::DerivedAccessPreparedProgram,
        Holder::DerivedAccessPreparedViews,
        Holder::DerivedAccessPrepareUseCase,
        Holder::AddedClosurePartitionsSchemas,
        Holder::AddedWorkspaceService,
    ];
    pub(super) fn path(self) -> &'static str {
        match self {
            Holder::AxisResources => file!("axis_resources/mod.rs"),
            Holder::AxisView => file!("axis_resources/view.rs"),
            Holder::ClosurePartitions => file!("closure_partitions.rs"),
            Holder::ExecutionLease => file!("execution/lease.rs"),
            Holder::Invocation => file!("invocation.rs"),
            Holder::InvocationIndex => file!("invocation/index.rs"),
            Holder::InvocationViews => file!("invocation/views/mod.rs"),
            Holder::Workspace => file!("workspace/mod.rs"),
            Holder::GraphView => "domain/operator/contract/graph_view.rs",
            Holder::DerivedAccessPrepare => file!("derived/prepare.rs"),
            Holder::DerivedAccessRequest => file!("service.rs"),
            Holder::DerivedAccessViews => file!("derived/request/mod.rs"),
            Holder::DerivedAccessRelations => file!("relations.rs"),
            Holder::DerivedAccessScope => file!("scope.rs"),
            Holder::DerivedAccessServices => file!("program/facade_runtime/types.rs"),
            Holder::DerivedAccessPreparation => file!("program/preparation/service.rs"),
            Holder::DerivedAccessPreparedProgram => file!("program/prepared_execution/types.rs"),
            Holder::DerivedAccessPreparedViews => file!("program/prepared_execution/service.rs"),
            Holder::DerivedAccessPrepareUseCase => "inbound/use_cases/prepare/use_case.rs",
            Holder::AddedClosurePartitionsSchemas => file!("closure_partitions/schemas.rs"),
            Holder::AddedWorkspaceService => file!("workspace/service.rs"),
        }
    }
    /// The authorities its source names, exhaustively.
    pub(super) fn holds(self) -> &'static [Authority] {
        match self {
            Holder::AxisResources
            | Holder::AxisView
            | Holder::ClosurePartitions
            | Holder::InvocationIndex
            | Holder::InvocationViews
            | Holder::Workspace => &[Authority::AcquiredView],
            Holder::ExecutionLease | Holder::GraphView => &[Authority::CachedGraph],
            Holder::Invocation => &[Authority::AcquiredView, Authority::CachedGraph],
            Holder::DerivedAccessPrepare
            | Holder::DerivedAccessRequest
            | Holder::DerivedAccessViews
            | Holder::DerivedAccessRelations
            | Holder::DerivedAccessScope
            | Holder::DerivedAccessServices
            | Holder::DerivedAccessPreparation
            | Holder::DerivedAccessPreparedProgram
            | Holder::DerivedAccessPreparedViews
            | Holder::DerivedAccessPrepareUseCase => &[Authority::DerivedAccess],
            Holder::AddedClosurePartitionsSchemas => &[Authority::AcquiredView],
            Holder::AddedWorkspaceService => &[Authority::AcquiredView],
        }
    }
}
