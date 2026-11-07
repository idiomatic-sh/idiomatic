//! Every derived kind, its production route and the controls that cover it. The match has no
//! wildcard: a fourteenth kind fails here until its route, lifecycle, identity and acquisition
//! controls are named.
use super::entry::Entry;
use ryo_derived::domain::request::types::DerivedKind;

const FAMILY: &str = "application/file_dag/program/facade_runtime/file_execution/distinct_integration/join/raw_scope/family";
const RAW: &str = "application/file_dag/program/facade_runtime/file_execution/distinct_integration/join/raw_scope/test.rs";
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
const RAW_EXITS: &[(&str, &str)] = &[(
    "lifecycle/test.rs",
    "every_family_releases_its_last_consumer_on_success_cancellation_and_error",
)];
pub(super) fn route(kind: DerivedKind) -> Route {
    let stored = |entries: &'static [Entry]| Route {
        entries,
        lifecycle: STORED,
        identity: IDENTITY,
    };
    match kind {
        DerivedKind::RawKeyOrder => Route {
            entries: &[Entry::FiniteRaw, Entry::WithRawOrder, Entry::EvaluateNative],
            lifecycle: RAW_EXITS,
            identity: IDENTITY,
        },
        DerivedKind::Equality => stored(&[
            Entry::AdmitView,
            Entry::AcquireStored,
            Entry::AcquireStoredWithRoute,
        ]),
        DerivedKind::PredicateMatches
        | DerivedKind::CandidatePostings
        | DerivedKind::DistinctRepresentatives => stored(&[
            Entry::SelectorPlan,
            Entry::Select,
            Entry::PrepareProgramFile,
            Entry::PrepareViewsWithSchemas,
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
/// The program-level cases that name the raw order's own success and failure exits.
pub(super) const RAW_CASES: (&str, &[&str]) = (
    RAW,
    &[
        "native_raw_streaming_releases_shared_ledger",
        "callback_failure_releases_raw_lease",
        "sink_failure_releases_raw_scope_and_publications",
        "raw_port_failure_is_not_rewritten",
    ],
);
