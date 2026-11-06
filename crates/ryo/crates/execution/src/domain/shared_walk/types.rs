//! File-local scratch for one borrowed relation pass.

use crate::domain::{
    operating::types::Memo, positions::types::ClassPlace, table_cells::types::ColumnPlaces,
};
use ryo_relations::domain::relations::types::Value;

pub(crate) mod prepared_walk;

/// Number of equivalence classes in one prepared request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ClassCount(pub usize);

/// Candidate source rows offered to a reached class.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RowsVisited(pub u64);

/// Immutable identity of an exact ordered predicate prefix in this request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct PredicatePlace(pub usize);

/// The number of kinds named by one prepared class.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct KindCount(pub usize);

/// Whether a class can be decided by the shared pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Eligibility {
    #[default]
    Declined,
    Ready,
}

/// Reusable file-local comparison work. No request plan or published rows live here.
#[derive(Debug, Default)]
pub(crate) struct ClassWork {
    pub result: Option<ClassResult>,
    pub visits: RowsVisited,
    pub kinds: KindCount,
    pub needed: ColumnPlaces,
    pub memo: Vec<Memo>,
    pub eligible: Eligibility,
}

/// Dense position in one admitted cohort, never a compiled class identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CohortPlace(pub usize);

#[derive(Debug, Default)]
pub(crate) struct WalkWork {
    pub active: Vec<ClassPlace>,
    pub work: Vec<ClassWork>,
    pub by_kind: Vec<Vec<CohortPlace>>,
    pub columns_by_kind: Vec<ColumnPlaces>,
    pub every: Vec<CohortPlace>,
    pub cells: Vec<Value>,
    pub predicates: std::collections::HashMap<
        PredicatePlace,
        Result<
            crate::domain::table_model::types::Verdict,
            crate::domain::operating::errors::OperatingError,
        >,
    >,
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
pub(crate) struct ClassResult {
    pub positions: crate::domain::positions::types::Slots,
    pub completion: Result<(), crate::domain::operating::errors::OperatingError>,
}

/// Read-only accounting of the bound physical program; never admission authority.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct WalkRouteCount(pub usize);
#[derive(Debug, Default)]
pub(crate) struct WalkCensus {
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
