//! File-local scratch for one borrowed relation pass.

use crate::domain::{
    operating::types::Memo, positions::types::ClassPlace, table_cells::types::ColumnPlaces,
    table_scratch::types::ComparisonNumbers,
};
use ryo_relations::domain::relations::types::Value;

pub(crate) mod prepared_walk;

/// The number of kinds named by one prepared class.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct KindCount(pub usize);

/// Whether the caller admitted a rule into this file's shared native pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RuleActivity {
    Active,
    Inactive,
}

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
    pub kinds: KindCount,
    pub rest: ComparisonNumbers,
    pub needed: ColumnPlaces,
    pub memo: Vec<Memo>,
    pub eligible: Eligibility,
}

#[derive(Debug, Default)]
pub(crate) struct WalkWork {
    pub work: Vec<ClassWork>,
    pub by_kind: Vec<Vec<ClassPlace>>,
    pub columns_by_kind: Vec<ColumnPlaces>,
    pub every: Vec<ClassPlace>,
    pub cells: Vec<Value>,
}

/// Actual outer shared native traversals in this file generation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WalkPassCount(pub u64);
/// Source positions visited by the outer traversal, independently of class visits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WalkSourceRows(pub u64);
/// Read-only work performed by the actual shared walk, never a schedule or capability.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WalkCounts {
    pub passes: WalkPassCount,
    pub source_rows: WalkSourceRows,
}

/// Accepted rows precede the first terminal predicate error in source order.
#[derive(Debug)]
pub(crate) struct ClassResult {
    pub positions: crate::domain::positions::types::Slots,
    pub completion: Result<(), crate::domain::operating::errors::OperatingError>,
}
