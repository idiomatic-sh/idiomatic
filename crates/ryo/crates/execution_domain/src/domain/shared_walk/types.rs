//! File-local scratch for one borrowed relation pass.

use crate::domain::{
    operating::errors::OperatingError, step_classes::types::ClassPlace,
    table_cells::types::ColumnPlaces,
};
use ryo_operators::domain::contract::types::PredicateOutcome;
use ryo_operators::domain::table_scratch::types::Memo;
use ryo_relations::domain::relations::types::Value;

pub mod native_batch;
pub mod prepared_walk;
pub mod walk_route;

/// Number of equivalence classes in one prepared request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ClassCount(pub usize);

/// Candidate source rows offered to a reached class.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RowsVisited(pub u64);

/// Immutable identity of an exact ordered predicate prefix in this request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PredicatePlace(pub usize);

/// Place of one comparison within its class's ordered predicates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PredicateNumber(pub usize);

/// A stored class result with the source rows its decision visited.
#[derive(Clone, Debug)]
pub struct ClassLease {
    /// The stored result of the class.
    pub result: std::sync::Arc<ClassResult>,
    /// How many source rows the class's decision visited.
    pub visits: RowsVisited,
}

/// The number of kinds named by one prepared class.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KindCount(pub usize);

/// Whether a class can be decided by the shared pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Eligibility {
    /// The class is not walked together with others.
    #[default]
    Declined,
    /// The class is ready for the shared walk.
    Ready,
}

/// Reusable file-local comparison work. No request plan or published rows live here.
#[derive(Debug, Default)]
pub struct ClassWork {
    /// The class's result, once decided.
    pub result: Option<ClassResult>,
    /// How many source rows the decision visited.
    pub visits: RowsVisited,
    /// How many kinds the class compares.
    pub kinds: KindCount,
    /// The columns the class reads.
    pub needed: ColumnPlaces,
    /// The memoized comparisons of the class.
    pub memo: Vec<Memo>,
    /// Whether the class takes part in the shared walk.
    pub eligible: Eligibility,
}

/// Dense position in one admitted cohort, never a compiled class identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CohortPlace(pub usize);

/// The reusable work of one shared walk over a file: its classes and the cells and decisions of the current row.
#[derive(Debug, Default)]
pub struct WalkWork {
    /// The classes still being decided.
    pub active: Vec<ClassPlace>,
    /// The work of each class.
    pub work: Vec<ClassWork>,
    /// The classes of each kind, by kind.
    pub by_kind: Vec<Vec<CohortPlace>>,
    /// The columns each kind's classes read, by kind.
    pub columns_by_kind: Vec<ColumnPlaces>,
    /// The classes of every kind.
    pub every: Vec<CohortPlace>,
    /// The cells of the current row.
    pub cells: Vec<Value>,
    /// The decisions of the current row's predicates.
    pub predicates: PredicateMemo,
}

/// Decisions of the current source row indexed by `PredicatePlace`, and the places filled.
#[derive(Debug, Default)]
pub struct PredicateMemo {
    /// The decision of each predicate, by its place, once made.
    pub slots: Vec<Option<Result<PredicateOutcome, OperatingError>>>,
    /// The places filled for the current row.
    pub filled: Vec<PredicatePlace>,
}

/// Actual outer shared native traversals in this file generation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WalkPassCount(pub u64);
/// Source positions visited by the outer traversal, independently of class visits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WalkSourceRows(pub u64);
/// Rows actually offered to a class predicate, including its first failing row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WalkPredicateRows(pub u64);
/// Actual cohort scratch resets and stored class result allocations.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WalkClassWork(pub u64);
/// Semantic predicate decisions actually performed after prefix sharing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WalkPredicateTests(pub u64);
/// Source rows actually decoded for demanded predicates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WalkDecodedRows(pub u64);
/// Read-only work performed by the actual shared walk, never a schedule or capability.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WalkCounts {
    /// How many passes the walk made.
    pub passes: WalkPassCount,
    /// How many source rows it read.
    pub source_rows: WalkSourceRows,
    /// How many rows its predicates were tested on.
    pub predicate_rows: WalkPredicateRows,
    /// How many predicate tests it ran.
    pub predicate_tests: WalkPredicateTests,
    /// How many predicate decisions it reused.
    pub predicate_reuses: WalkPredicateTests,
    /// How many rows it decoded.
    pub decoded_rows: WalkDecodedRows,
    /// How many classes kept scratch work.
    pub scratch_classes: WalkClassWork,
    /// How many classes kept a result slot.
    pub result_slots: WalkClassWork,
}

/// Accepted rows precede the first terminal predicate error in source order.
#[derive(Debug)]
pub struct ClassResult {
    /// The positions of the accepted rows.
    pub positions: Slots,
    /// Whether the walk completed, or the first terminal predicate error.
    pub completion: Result<(), crate::domain::operating::errors::OperatingError>,
}

/// Read-only accounting of the bound physical program; never admission authority.
#[derive(Clone, Copy, Debug, Default)]
pub struct WalkRouteCount(pub usize);
/// How the instructions of a request were routed to the shared walk, counted by route.
#[derive(Debug, Default)]
pub struct WalkCensus {
    /// How many instructions there were.
    pub instructions: WalkRouteCount,
    /// How many went to the shared walk.
    pub shared: WalkRouteCount,
    /// The classes the shared instructions formed.
    pub classes: Vec<ClassPlace>,
    /// How many were declined by their representation.
    pub representation: WalkRouteCount,
    /// How many depend on another instruction's rows.
    pub dependent: WalkRouteCount,
    /// How many read an observation.
    pub observation: WalkRouteCount,
    /// How many read a row-varying input.
    pub row: WalkRouteCount,
    /// How many read a derived view.
    pub derived: WalkRouteCount,
    /// How many read a request source.
    pub request: WalkRouteCount,
    /// How many continue another instruction.
    pub continuation: WalkRouteCount,
}

/// A declined representation is emitted only by the semantic preflight that inspected it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalkRefusal {
    /// The representation was declined by the semantic preflight.
    Representation,
}

/// The positions of the elements a walk step admits, row after row.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Slots(pub Vec<u32>);
