//! Plain selector program and cache data, independent of borrowed provider storage.

use ryo_language::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::sync::Arc;

pub(crate) mod program;

/// A pure ordered predicate program with an independently composable output map.
#[derive(Clone, Debug)]
pub(crate) struct Selector {
    /// Original stored population.
    pub relation: Relation,
    /// Ordered physical predicates and their referenced request values.
    pub identity: Arc<crate::domain::table_model::types::SelectionIdentity>,
    /// Physical columns corresponding to the step's output.
    pub columns: Vec<ColumnIndex>,
    /// Drop duplicate projected tuples at this terminal step only.
    pub distinct: bool,
}

/// Immutable selector descriptors for one plan and its request bindings.
#[derive(Debug)]
pub(crate) struct PreparedPlan {
    /// Same request-local plan identity as its layout.
    pub identity: SelectionPlanId,
    /// Fixture setup compares explicit bindings; production reuses the canonical request Arc.
    #[cfg(test)]
    pub bound: Vec<crate::domain::table_model::types::ParameterBinding>,
    /// Eligible direct/chain selectors and terminal projections by step index.
    pub steps: Vec<Option<Arc<Selector>>>,
}

/// Request-local plan address used only to reuse its immutable selector descriptors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SelectionPlanId(pub usize);

/// A successful match-ID result; projections deliberately do not form part of its key.
#[derive(Debug)]
pub(crate) struct Matches {
    /// The pure program that established this result.
    pub selector: Arc<Selector>,
    /// Actual provider schema, compared on every lookup.
    pub kinds: &'static [CellKind],
    /// Physical row population, compared on every lookup.
    pub population: usize,
    /// Original positions in ascending source order.
    pub rows: Arc<Vec<u32>>,
}

/// One complete, immutable physical column routed to only the literal texts requested
/// by this file's active selectors. Row positions in each posting remain in source order.
#[derive(Debug)]
pub(crate) struct LiteralFanout {
    pub relation: Relation,
    pub column: ColumnIndex,
    pub kinds: &'static [CellKind],
    pub population: usize,
    pub source_bits: usize,
    pub literals: Vec<LiteralPostings>,
}

#[derive(Debug)]
pub(crate) struct LiteralPostings {
    pub text: String,
    pub rows: Arc<Vec<u32>>,
}

/// One discriminator column used by distinct active predicate programs.
#[derive(Debug)]
pub(crate) struct CandidateColumn {
    /// Stored population.
    pub relation: Relation,
    /// Physical equality column.
    pub column: ColumnIndex,
    /// First program seen, for exact structural comparison.
    pub first: Arc<Selector>,
    /// Another distinct predicate/binding sequence can reuse its index.
    pub repeated: bool,
}

/// Aggregate execution counts; never one event per row.
#[derive(Debug, Default)]
pub(crate) struct SelectionCounts {
    /// Actual source positions visited while scanning or indexing.
    pub scanned: u64,
    /// Positions admitted by an equality discriminator.
    pub candidates: u64,
    /// Predicate cells decoded.
    pub predicate_cells: u64,
    /// Cells decoded during compact replay through the row-stream boundary.
    pub output_cells: u64,
    /// Cells appended to materialized collectors during this file's execution.
    pub collected_cells: u64,
    /// Successful physical selection builds.
    pub builds: u64,
    /// Replayed successful selections.
    pub reuses: u64,
    /// Successful selections whose retention was refused.
    pub refusals: u64,
}

/// Diagnostic-only counts for one prepared selector in one file generation.
/// No row data or provider reference is retained here.
#[derive(Debug)]
pub(crate) struct SelectorHistogram {
    pub selector: Arc<Selector>,
    pub calls: u64,
    pub builds: u64,
    pub reuses: u64,
    pub errors: u64,
    pub matched_rows: u64,
    pub full_scan_rows: u64,
    pub index_build_rows: u64,
    pub indexed_candidates: u64,
    pub predicate_cells: u64,
    pub paths: [u64; 9],
    pub admission: [u64; 4],
}

impl SelectorHistogram {
    pub(crate) fn new(selector: &Arc<Selector>) -> Self {
        Self {
            selector: Arc::clone(selector),
            calls: 0,
            builds: 0,
            reuses: 0,
            errors: 0,
            matched_rows: 0,
            full_scan_rows: 0,
            index_build_rows: 0,
            indexed_candidates: 0,
            predicate_cells: 0,
            paths: [0; 9],
            admission: [0; 4],
        }
    }
}

/// Prepared construction contracts indexed by exact request-owned selector identity.
#[derive(Debug, Default)]
pub(crate) struct PreparedSelectionViews {
    /// Pins every selector address for the complete request lifetime.
    pub selectors: Vec<Arc<Selector>>,
    /// O(1) capability lookup; no metadata construction occurs while selecting.
    pub by_selector: std::collections::HashMap<usize, Arc<PreparedSelectorViews>>,
    pub groups: usize,
}
#[derive(Debug)]
pub(crate) struct PreparedSelectorViews {
    pub group: usize,
    pub predicate: Arc<crate::domain::derived_views::types::DerivedContract>,
    pub postings: Arc<crate::domain::derived_views::types::DerivedContract>,
    pub distinct: Arc<crate::domain::derived_views::types::DerivedContract>,
    /// Every statically possible equality discriminator, chosen without runtime preparation.
    pub equalities: std::collections::HashMap<
        ColumnIndex,
        Arc<crate::domain::derived_views::types::DerivedContract>,
    >,
}
