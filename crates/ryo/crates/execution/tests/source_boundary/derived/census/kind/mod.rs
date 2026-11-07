//! Every derived kind, its production route and the controls that cover it. The match has no
//! wildcard: a tenth kind fails here until its route, lifecycle, identity and acquisition
//! controls are named.
use super::entry::Entry;
use ryo_derived::domain::request::types::DerivedKind;

const FAMILY: &str =
    "application/file_dag/program/facade_runtime/file_execution/distinct_integration/join/family";
pub(super) struct Route {
    /// The owner entries its production route acquires through.
    pub(super) entries: &'static [Entry],
    /// `(file, test)`: every exit (success, cancellation, error) through the production entry.
    pub(super) lifecycle: &'static [(&'static str, &'static str)],
    /// Mismatched generation, plan, population and descriptor refusals.
    pub(super) identity: (&'static str, &'static str),
}
const STORED: &[(&str, &str)] = &[
    (
        "lifecycle/test.rs",
        "every_family_leaves_through_its_production_entry_on_success_cancellation_and_error",
    ),
    (
        "lifecycle/test.rs",
        "every_family_releases_its_last_consumer_on_success_cancellation_and_error",
    ),
];
const IDENTITY: (&str, &str) = (
    "identity/test.rs",
    "every_family_refuses_every_mismatched_capability",
);
pub(super) fn route(kind: DerivedKind) -> Route {
    let stored = |entries: &'static [Entry]| Route {
        entries,
        lifecycle: STORED,
        identity: IDENTITY,
    };
    match kind {
        // File setup admits the owner backend every stored family is acquired from.
        DerivedKind::Equality => stored(&[
            Entry::PrepareProgramFile,
            Entry::AdmitView,
            Entry::AcquireStored,
            Entry::AcquireStoredWithRoute,
        ]),
        DerivedKind::Adjacency => stored(&[
            Entry::PrepareGraph,
            Entry::AcquireGraph,
            Entry::DemandAnswers,
            Entry::GraphRow,
            Entry::Reach,
            Entry::Axis,
        ]),
        DerivedKind::Membership => stored(&[
            Entry::AcquireGraphDomain,
            Entry::DemandAnswers,
            Entry::Nearest,
        ]),
        DerivedKind::PartitionOrder => stored(&[Entry::AdmitView, Entry::AcquireStored]),
        DerivedKind::ClosureAdjacency => stored(&[Entry::AdmitPartition, Entry::ScopedBuild]),
        DerivedKind::CycleAdjacency | DerivedKind::OrderedKeys | DerivedKind::OrderedDfs => {
            stored(&[Entry::AdmitView, Entry::AcquireStored])
        }
        DerivedKind::NativeLookup => stored(&[Entry::AdmitView, Entry::TryAcquireRows]),
    }
}
pub(super) fn family_file(name: &str) -> String {
    format!("{FAMILY}/{name}")
}
