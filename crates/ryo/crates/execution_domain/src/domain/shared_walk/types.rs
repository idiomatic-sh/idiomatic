//! File-local scratch for one borrowed relation pass.

use crate::domain::{
    operating::{errors::OperatingError, types::Memo},
    step_classes::types::ClassPlace,
    table_cells::types::ColumnPlaces,
    table_model::types::Verdict,
};
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
    pub result: std::sync::Arc<ClassResult>,
    pub visits: RowsVisited,
}

/// The number of kinds named by one prepared class.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KindCount(pub usize);

/// Whether a class can be decided by the shared pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Eligibility {
    #[default]
    Declined,
    Ready,
}

/// Reusable file-local comparison work. No request plan or published rows live here.
#[derive(Debug, Default)]
pub struct ClassWork {
    pub result: Option<ClassResult>,
    pub visits: RowsVisited,
    pub kinds: KindCount,
    pub needed: ColumnPlaces,
    pub memo: Vec<Memo>,
    pub eligible: Eligibility,
}

/// Dense position in one admitted cohort, never a compiled class identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CohortPlace(pub usize);

#[derive(Debug, Default)]
pub struct WalkWork {
    pub active: Vec<ClassPlace>,
    pub work: Vec<ClassWork>,
    pub by_kind: Vec<Vec<CohortPlace>>,
    pub columns_by_kind: Vec<ColumnPlaces>,
    pub every: Vec<CohortPlace>,
    pub cells: Vec<Value>,
    pub predicates: PredicateMemo,
}

/// Decisions of the current source row indexed by `PredicatePlace`, and the places filled.
#[derive(Debug, Default)]
pub struct PredicateMemo {
    pub slots: Vec<Option<Result<Verdict, OperatingError>>>,
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
    pub passes: WalkPassCount,
    pub source_rows: WalkSourceRows,
    pub predicate_rows: WalkPredicateRows,
    pub predicate_tests: WalkPredicateTests,
    pub predicate_reuses: WalkPredicateTests,
    pub decoded_rows: WalkDecodedRows,
    pub scratch_classes: WalkClassWork,
    pub result_slots: WalkClassWork,
}

/// Accepted rows precede the first terminal predicate error in source order.
#[derive(Debug)]
pub struct ClassResult {
    pub positions: Slots,
    pub completion: Result<(), crate::domain::operating::errors::OperatingError>,
}

/// Read-only accounting of the bound physical program; never admission authority.
#[derive(Clone, Copy, Debug, Default)]
pub struct WalkRouteCount(pub usize);
#[derive(Debug, Default)]
pub struct WalkCensus {
    pub instructions: WalkRouteCount,
    pub shared: WalkRouteCount,
    pub classes: Vec<ClassPlace>,
    pub representation: WalkRouteCount,
    pub dependent: WalkRouteCount,
    pub observation: WalkRouteCount,
    pub row: WalkRouteCount,
    pub derived: WalkRouteCount,
    pub request: WalkRouteCount,
    pub continuation: WalkRouteCount,
}

/// A declined representation is emitted only by the semantic preflight that inspected it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalkRefusal {
    Representation,
}

/// The positions of the elements a walk step admits, row after row.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Slots(pub Vec<u32>);
