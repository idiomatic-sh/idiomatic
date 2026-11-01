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
}

/// Immutable selector descriptors for one plan and its request bindings.
#[derive(Debug)]
pub(crate) struct PreparedPlan {
    /// Same request-local plan identity as its layout.
    pub identity: SelectionPlanId,
    /// Request values that determine the prepared program, checked before reuse across visits.
    pub bound: Vec<crate::domain::table_model::types::ParameterBinding>,
    /// Eligible direct/chain selectors and non-distinct projections by step index.
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
