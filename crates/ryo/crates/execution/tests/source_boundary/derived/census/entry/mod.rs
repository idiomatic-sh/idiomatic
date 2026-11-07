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
            Authority::AcquiredView => ("AcquiredView", &c::authority::AUTHORITY_ACQUIRED),
            Authority::ResourceDependency => ("ResourceDependency", &c::authority::AUTHORITY_DEPENDENCY),
            Authority::GraphHandle => ("GraphHandle", &c::authority::AUTHORITY_GRAPH),
            Authority::DomainHandle => ("DomainHandle", &c::authority::AUTHORITY_DOMAIN),
            Authority::RootAnswer => ("RootAnswer", &c::authority::AUTHORITY_ANSWER),
            Authority::CachedGraph => ("CachedGraph", &c::authority::AUTHORITY_CACHED),
            Authority::DerivedAccess => ("DerivedAccess", &c::authority::AUTHORITY_ACCESS),
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
const ADMISSION: &str =
    "crate::application::file_dag::derived::owner::acquisition::admission::service";
const HANDLES: &str =
    "crate::application::file_dag::derived::owner::graphs::types::handles::service";
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
    PrepareGraph,
    AcquireGraph,
    AcquireGraphDomain,
    GraphRow,
    DemandAnswers,
    Reach,
    Axis,
    Nearest,
    PrepareProgramFile,
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
    Wanted,
}
impl Entry {
    pub(super) const ALL: [Entry; 46] = [
        Entry::AdmitView,
        Entry::AdmitPartition,
        Entry::AcquireStored,
        Entry::AcquireStoredWithRoute,
        Entry::TryAcquireRows,
        Entry::ScopedBuild,
        Entry::PrepareGraph,
        Entry::AcquireGraph,
        Entry::AcquireGraphDomain,
        Entry::GraphRow,
        Entry::DemandAnswers,
        Entry::Reach,
        Entry::Axis,
        Entry::Nearest,
        Entry::PrepareProgramFile,
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
        Entry::Wanted,
    ];
}

mod rows;
