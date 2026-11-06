//! The exits and entries a family is driven through, and the release every exit must end with.
use ryo_derived::domain::contract::types::DerivedKind;

/// How the last consumer of a family leaves through its production entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag) enum Exit {
    /// The view is acquired, read and dropped.
    Succeeds,
    /// The demand is withdrawn after the request is prepared and before the view is read.
    Cancelled,
    /// The production entry refuses; nothing acquired may stay charged.
    Errors,
}
/// The production entry a family is acquired through.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag) enum Entry {
    /// `InputAdmissions::{acquire_derived, try_native_lookup}` over an invocation workspace.
    Workspace,
    /// The `PartitionedClosure` cursor of `closure_partitions`.
    Partition,
    /// `DerivedOwner::select` over a stored relation.
    Selection,
    /// `acquire_graph` and `acquire_graph_domain` over an admitted graph input.
    Graph,
    /// `finite_raw` and `with_raw_order`, which require a `KernelPermit`.
    Permit,
}
/// Exhaustive on purpose: a new family must name its production entry before this file compiles.
pub(in crate::application::file_dag) fn entry(kind: DerivedKind) -> Entry {
    match kind {
        DerivedKind::RawKeyOrder => Entry::Permit,
        DerivedKind::Equality
        | DerivedKind::CycleAdjacency
        | DerivedKind::OrderedKeys
        | DerivedKind::OrderedDfs
        | DerivedKind::NativeLookup => Entry::Workspace,
        DerivedKind::PartitionOrder | DerivedKind::ClosureAdjacency => Entry::Partition,
        DerivedKind::PredicateMatches
        | DerivedKind::CandidatePostings
        | DerivedKind::DistinctRepresentatives => Entry::Selection,
        DerivedKind::Adjacency | DerivedKind::Membership => Entry::Graph,
    }
}
pub(in crate::application::file_dag::derived::owner::acquisition::family::fixtures::route) fn released(kind: DerivedKind, exit: Exit, ledger: (usize, usize, usize)) {
    assert_eq!(
        ledger,
        (0, 0, 0),
        "{kind:?} {exit:?}: every lease retires with the owner generation"
    );
}
/// The exit itself: the handles and results of the production entry are dropped and the generation
/// is still live, so nothing but the entry's own release can have returned the required bytes.
pub(in crate::application::file_dag::derived::owner::acquisition::family::fixtures::route) fn released_at_exit(kind: DerivedKind, exit: Exit, ledger: (usize, usize, usize)) {
    assert_eq!(
        ledger.0, 0,
        "{kind:?} {exit:?}: the exit itself releases the required lease, before the generation retires"
    );
}
