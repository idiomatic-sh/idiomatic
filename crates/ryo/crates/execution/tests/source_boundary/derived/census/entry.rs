//! Every function of the derived owner a consumer outside it can name, and what controls it.
//! `row` is an exhaustive match with no wildcard: a new owner function fails the census until it
//! is classified, and the owner surface read from source must equal `ALL`.
use super::controls::{self as c, Acquire, Negative};

macro_rules! owner {
    ($path:literal) => {
        concat!("application/file_dag/derived/owner/", $path)
    };
}
macro_rules! file {
    ($path:literal) => {
        concat!("application/file_dag/", $path)
    };
}
pub(super) enum Authority {
    AcquiredView,
    ResourceDependency,
    GraphHandle,
    DomainHandle,
    RootAnswer,
    CachedGraph,
    DerivedAccess,
}
impl Authority {
    pub(super) const ALL: [Authority; 7] = [
        Authority::AcquiredView,
        Authority::ResourceDependency,
        Authority::GraphHandle,
        Authority::DomainHandle,
        Authority::RootAnswer,
        Authority::CachedGraph,
        Authority::DerivedAccess,
    ];
    /// The identifier a holder's source names, and the forgery control of the authority.
    pub(super) fn control(&self) -> (&'static str, &'static Negative) {
        match self {
            Authority::AcquiredView => ("AcquiredView", &c::AUTHORITY_ACQUIRED),
            Authority::ResourceDependency => ("ResourceDependency", &c::AUTHORITY_DEPENDENCY),
            Authority::GraphHandle => ("GraphHandle", &c::AUTHORITY_GRAPH),
            Authority::DomainHandle => ("DomainHandle", &c::AUTHORITY_DOMAIN),
            Authority::RootAnswer => ("RootAnswer", &c::AUTHORITY_ANSWER),
            Authority::CachedGraph => ("CachedGraph", &c::AUTHORITY_CACHED),
            Authority::DerivedAccess => ("DerivedAccess", &c::AUTHORITY_ACCESS),
        }
    }
}
pub(super) enum Class {
    /// Acquires, admits or demands a view: a positive and a negative compile control.
    Acquires(&'static Acquire),
    /// Reads a leased view or handle: the accessor is nameable; the authority cannot be forged.
    Reads {
        authority: Authority,
        owner_type: &'static str,
        path: &'static str,
    },
    /// No view crosses: accounting, lifecycle or diagnostics.
    Exempt(&'static str),
}
pub(super) struct Row {
    pub(super) file: &'static str,
    pub(super) name: &'static str,
    pub(super) class: Class,
    /// Production files outside the owner that call it, as read from source.
    pub(super) sites: &'static [&'static str],
    pub(super) call: &'static str,
    pub(super) with: &'static str,
}
fn acquires(
    file: &'static str,
    name: &'static str,
    control: &'static Acquire,
    sites: &'static [&'static str],
    call: &'static str,
    with: &'static str,
) -> Row {
    Row {
        file,
        name,
        class: Class::Acquires(control),
        sites,
        call,
        with,
    }
}
const ADMISSION: &str = "crate::application::file_dag::derived::owner::acquisition::admission::service";
const HANDLES: &str = "crate::application::file_dag::derived::owner::graphs::types::handles::service";
fn reads(
    file: &'static str,
    name: &'static str,
    authority: Authority,
    owner_type: &'static str,
    path: &'static str,
) -> Row {
    Row {
        file,
        name,
        class: Class::Reads {
            authority,
            owner_type,
            path,
        },
        sites: &[],
        call: "",
        with: "",
    }
}
fn exempt(file: &'static str, name: &'static str, reason: &'static str) -> Row {
    Row {
        file,
        name,
        class: Class::Exempt(reason),
        sites: &[],
        call: "",
        with: "",
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Entry {
    AdmitView,
    AdmitPartition,
    AcquireStored,
    AcquireStoredWithRoute,
    TryAcquireRows,
    ScopedBuild,
    FiniteRaw,
    WithRawOrder,
    EvaluateNative,
    PrepareGraph,
    AcquireGraph,
    AcquireGraphDomain,
    GraphRow,
    DemandAnswers,
    Reach,
    Axis,
    Nearest,
    SelectorPlan,
    Select,
    PrepareProgramFile,
    PrepareViewsWithSchemas,
    ViewRead,
    ContractRead,
    ConstructionRead,
    DependencyRead,
    DependencyBackings,
    GraphRead,
    OrientationRead,
    MembersRead,
    ValidateOwner,
    RootValues,
    Counts,
    ForPredicate,
    MetadataBytes,
    OwnerNew,
    AccessPrepare,
    OwnedBytes,
    PeakRequiredViews,
    PeakWorkingBytes,
    ReleasedEntries,
    RequiredBackendOverlap,
    RetainedBytes,
    WorkingBytes,
    Report,
    SlotReplace,
    SlotRead,
    OptionalLease,
    RequiredLease,
    Clear,
    Pin,
    Release,
    SelectorWork,
    Wanted,
}
impl Entry {
    pub(super) const ALL: [Entry; 53] = [
        Entry::AdmitView,
        Entry::AdmitPartition,
        Entry::AcquireStored,
        Entry::AcquireStoredWithRoute,
        Entry::TryAcquireRows,
        Entry::ScopedBuild,
        Entry::FiniteRaw,
        Entry::WithRawOrder,
        Entry::EvaluateNative,
        Entry::PrepareGraph,
        Entry::AcquireGraph,
        Entry::AcquireGraphDomain,
        Entry::GraphRow,
        Entry::DemandAnswers,
        Entry::Reach,
        Entry::Axis,
        Entry::Nearest,
        Entry::SelectorPlan,
        Entry::Select,
        Entry::PrepareProgramFile,
        Entry::PrepareViewsWithSchemas,
        Entry::ViewRead,
        Entry::ContractRead,
        Entry::ConstructionRead,
        Entry::DependencyRead,
        Entry::DependencyBackings,
        Entry::GraphRead,
        Entry::OrientationRead,
        Entry::MembersRead,
        Entry::ValidateOwner,
        Entry::RootValues,
        Entry::Counts,
        Entry::ForPredicate,
        Entry::MetadataBytes,
        Entry::OwnerNew,
        Entry::AccessPrepare,
        Entry::OwnedBytes,
        Entry::PeakRequiredViews,
        Entry::PeakWorkingBytes,
        Entry::ReleasedEntries,
        Entry::RequiredBackendOverlap,
        Entry::RetainedBytes,
        Entry::WorkingBytes,
        Entry::Report,
        Entry::SlotReplace,
        Entry::SlotRead,
        Entry::OptionalLease,
        Entry::RequiredLease,
        Entry::Clear,
        Entry::Pin,
        Entry::Release,
        Entry::SelectorWork,
        Entry::Wanted,
    ];
    pub(super) fn row(self) -> Row {
        let admission = owner!("acquisition/admission/service/issuance/service.rs");
        let acquisition = owner!("acquisition/service.rs");
        let raw = owner!("raw_scope.rs");
        let graph = owner!("graph.rs");
        let roots = owner!("graphs/types/roots/value.rs");
        let selection = owner!("selection/mod.rs");
        let setup = owner!("setup.rs");
        let view = owner!("acquisition/admission/service.rs");
        let handles = owner!("graphs/types/handles/service.rs");
        let owner = "application/file_dag/derived/owner.rs";
        let accounting = "accounting: bytes and counts the owner reports, no view crosses";
        // The graph demand lives under the owner: no file outside it calls these entries.
        let cached: &'static [&'static str] = &[];
        match self {
            Entry::AdmitView => acquires(admission, "admit_view", &c::ADMIT_VIEW, &[file!("closure_partitions.rs"), file!("relations/selected_access.rs"), file!("workspace.rs")], ".admit_view(", ""),
            Entry::AdmitPartition => acquires(admission, "admit_partition", &c::ADMIT_PARTITION, &[file!("closure_partitions.rs")], ".admit_partition(", ""),
            Entry::AcquireStored => acquires(acquisition, "acquire_stored", &c::ACQUIRE_STORED, &[file!("closure_partitions.rs"), file!("workspace.rs")], ".acquire_stored(", ""),
            Entry::AcquireStoredWithRoute => acquires(acquisition, "acquire_stored_with_route", &c::ACQUIRE_STORED_WITH_ROUTE, &[file!("relations/selected_access.rs")], ".acquire_stored_with_route(", ""),
            Entry::TryAcquireRows => acquires(acquisition, "try_acquire_rows", &c::TRY_ACQUIRE_ROWS, &[file!("workspace.rs")], ".try_acquire_rows(", ""),
            Entry::ScopedBuild => acquires(owner!("acquisition/scoped/service.rs"), "scoped_build", &c::SCOPED_BUILD, &[file!("closure_partitions.rs")], ".scoped_build(", ""),
            Entry::FiniteRaw => acquires(raw, "finite_raw", &c::FINITE_RAW, &[file!("program/kernel_session/positions/service.rs")], ".finite_raw(", ""),
            Entry::WithRawOrder => acquires(raw, "with_raw_order", &c::WITH_RAW_ORDER, &[], ".with_raw_order(", ""),
            Entry::EvaluateNative => acquires(raw, "evaluate_native", &c::EVALUATE_NATIVE, &[file!("program/kernel_session/positions/service.rs")], ".evaluate_native(", ""),
            Entry::PrepareGraph => acquires(graph, "prepare_graph", &c::PREPARE_GRAPH, &[file!("invocation.rs")], ".prepare_graph(", ""),
            Entry::AcquireGraph => acquires(graph, "acquire_graph", &c::ACQUIRE_GRAPH, cached, ".acquire_graph(", ""),
            Entry::AcquireGraphDomain => acquires(graph, "acquire_graph_domain", &c::ACQUIRE_GRAPH_DOMAIN, cached, ".acquire_graph_domain(", ""),
            Entry::GraphRow => acquires(graph, "graph_row", &c::GRAPH_ROW, &[file!("execution/lease.rs")], ".graph_row(", ""),
            Entry::DemandAnswers => acquires(roots, "new", &c::DEMAND_ANSWERS, cached, "DemandAnswers::new(", ""),
            Entry::Reach => acquires(roots, "reach", &c::REACH, cached, ".reach(", "DemandAnswers"),
            Entry::Axis => acquires(roots, "axis", &c::AXIS, cached, ".axis(", "DemandAnswers"),
            Entry::Nearest => acquires(roots, "nearest", &c::NEAREST, cached, ".nearest(", "DemandAnswers"),
            Entry::SelectorPlan => acquires(selection, "selector_plan", &c::SELECTOR_PLAN, &[file!("program/selection.rs")], ".selector_plan(", ""),
            Entry::Select => acquires(selection, "select", &c::SELECT, &[file!("program/selection.rs")], ".select(", "derived"),
            Entry::PrepareProgramFile => acquires(setup, "prepare_program_file", &c::PREPARE_PROGRAM_FILE, &[file!("program/facade_runtime/file_execution.rs")], ".prepare_program_file(", ""),
            Entry::PrepareViewsWithSchemas => acquires(owner!("selection/preparation/service.rs"), "prepare_views_with_schemas", &c::PREPARE_VIEWS, &[file!("request.rs")], "prepare_views_with_schemas(", ""),
            Entry::ViewRead => reads(view, "view", Authority::AcquiredView, "AcquiredView", ADMISSION),
            Entry::ContractRead => reads(view, "contract", Authority::AcquiredView, "AcquiredView", ADMISSION),
            Entry::ConstructionRead => reads(view, "construction", Authority::AcquiredView, "AcquiredView", ADMISSION),
            Entry::DependencyRead => reads(view, "dependency", Authority::AcquiredView, "AcquiredView", ADMISSION),
            Entry::DependencyBackings => reads(view, "backings", Authority::ResourceDependency, "ResourceDependency", ADMISSION),
            Entry::GraphRead => reads(handles, "graph", Authority::GraphHandle, "GraphHandle", HANDLES),
            Entry::OrientationRead => reads(handles, "orientation", Authority::GraphHandle, "GraphHandle", HANDLES),
            Entry::MembersRead => reads(handles, "members", Authority::DomainHandle, "DomainHandle", HANDLES),
            Entry::ValidateOwner => reads(handles, "validate_owner", Authority::GraphHandle, "GraphHandle", HANDLES),
            Entry::RootValues => reads(owner!("graphs/types/handles/service.rs"), "values", Authority::RootAnswer, "RootAnswer", HANDLES),
            Entry::Counts => exempt(owner, "counts", accounting),
            Entry::ForPredicate => exempt(owner, "for_predicate", "a context key built from a prepared plan; no view crosses"),
            Entry::MetadataBytes => exempt(owner, "metadata_bytes", accounting),
            Entry::OwnerNew => exempt(owner, "new", "constructs the owner; the relation owner holds it"),
            Entry::AccessPrepare => exempt(owner!("types.rs"), "prepare", "request-time metadata through the owner-issued capability; the builder behind it is owner-private (AUTHORITY_ACCESS, SEALED_PORT)"),
            Entry::OwnedBytes => exempt(owner, "owned_bytes", accounting),
            Entry::PeakRequiredViews => exempt(owner, "peak_required_views", accounting),
            Entry::PeakWorkingBytes => exempt(owner, "peak_working_bytes", accounting),
            Entry::ReleasedEntries => exempt(owner, "released_entries", accounting),
            Entry::RequiredBackendOverlap => exempt(owner, "required_backend_overlap", accounting),
            Entry::RetainedBytes => exempt(owner, "retained_bytes", accounting),
            Entry::WorkingBytes => exempt(owner, "working_bytes", accounting),
            Entry::Report => exempt(owner!("diagnostics.rs"), "report", "diagnostic report of counts"),
            Entry::SlotReplace => exempt(owner!("graphs/service.rs"), "replace", "slot helper of the owner's own graph tables"),
            Entry::SlotRead => exempt(owner!("graphs/service.rs"), "slot", "slot helper of the owner's own graph tables"),
            Entry::OptionalLease => exempt(owner!("leases.rs"), "optional_lease", "ledger lease issued to publications, not a view"),
            Entry::RequiredLease => exempt(owner!("leases.rs"), "required_lease", "ledger lease issued to publications, not a view"),
            Entry::Clear => exempt(owner!("lifetime.rs"), "clear", "generation retirement; releases every view"),
            Entry::Pin => exempt(owner!("lifetime.rs"), "pin", "population pin; the lifecycle controls release it"),
            Entry::Release => exempt(owner!("lifetime.rs"), "release", "population release; the lifecycle controls drive it"),
            Entry::SelectorWork => exempt(owner!("observations.rs"), "selector_work", accounting),
            Entry::Wanted => exempt(setup, "wanted", "request-side relation list; no view crosses"),
        }
    }
}
