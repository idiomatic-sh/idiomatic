//! Plain selector program and cache data, independent of borrowed provider storage.

use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::plan_parts::types::{ColumnIndex, PlanComparison, RepeatedRows};
use ryo_values::domain::query_bindings::types::ParameterBinding;
use std::sync::Arc;

pub(crate) mod program;

/// Original row positions of a selection, shared through the admitted view that holds them.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SelectedPositions(pub Arc<Vec<u32>>);

impl std::ops::Deref for SelectedPositions {
    type Target = Arc<Vec<u32>>;
    fn deref(&self) -> &Arc<Vec<u32>> {
        &self.0
    }
}

/// Immutable semantic identity shared by prepared selectors and compact graph populations.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct SelectionIdentity {
    /// Ordered predicates over physical provider columns.
    pub comparisons: Vec<PlanComparison>,
    /// Only the binding values referenced by these predicates.
    pub bindings: Vec<ParameterBinding>,
}

/// A pure ordered predicate program with an independently composable output map.
#[derive(Clone, Debug)]
pub(crate) struct Selector {
    /// Original stored population.
    pub relation: Relation,
    /// Ordered physical predicates and their referenced request values.
    pub identity: Arc<SelectionIdentity>,
    /// Physical columns corresponding to the step's output.
    pub columns: Vec<ColumnIndex>,
    /// Drop duplicate projected tuples at this terminal step only.
    pub distinct: RepeatedRows,
}

/// Immutable selector descriptors for one plan and its request bindings.
#[derive(Debug)]
pub(crate) struct PreparedPlan {
    /// Eligible direct/chain selectors and terminal projections by step index.
    pub steps: Vec<Option<Arc<Selector>>>,
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
    /// Whether another distinct predicate/binding sequence can reuse its index.
    pub repeat: CandidateRepeat,
}

/// Whether a discriminator column is used by more than one distinct predicate/binding sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CandidateRepeat {
    /// Only one sequence uses the column.
    Single,
    /// Another distinct sequence can reuse its index.
    Repeated,
}

/// Actual source positions visited while scanning or indexing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ScannedRows(pub u64);

/// Positions admitted by an equality discriminator.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct AdmittedCandidates(pub u64);

/// Predicate cells decoded.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct DecodedPredicateCells(pub u64);

/// Actual reached predicate rows, including failed comparisons.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ReachedPredicateRows(pub u64);

/// Cells decoded during compact replay through the row-stream boundary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct DecodedOutputCells(pub u64);

/// Cells appended to materialized collectors during this file's execution.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct CollectedCells(pub u64);

/// Successful physical selection builds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SelectionBuilds(pub u64);

/// Replayed successful selections.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SelectionReuses(pub u64);

/// Successful selections whose retention was refused.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SelectionRefusals(pub u64);

/// Aggregate execution counts; never one event per row.
#[derive(Debug, Default)]
pub(crate) struct SelectionCounts {
    /// Actual source positions visited while scanning or indexing.
    pub scanned: ScannedRows,
    /// Positions admitted by an equality discriminator.
    pub candidates: AdmittedCandidates,
    /// Predicate cells decoded.
    pub predicate_cells: DecodedPredicateCells,
    /// Actual reached predicate rows, including failed comparisons.
    pub predicate_rows: ReachedPredicateRows,
    /// Cells decoded during compact replay through the row-stream boundary.
    pub output_cells: DecodedOutputCells,
    /// Cells appended to materialized collectors during this file's execution.
    pub collected_cells: CollectedCells,
    /// Successful physical selection builds.
    pub builds: SelectionBuilds,
    /// Replayed successful selections.
    pub reuses: SelectionReuses,
    /// Successful selections whose retention was refused.
    pub refusals: SelectionRefusals,
}

/// Prepared-selector acquisitions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SelectorCalls(pub u64);

/// Prepared-selector acquisitions that failed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SelectorErrors(pub u64);

/// Rows a prepared selector matched.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct MatchedRows(pub u64);

/// Rows a prepared selector visited by a full scan.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct FullScanRows(pub u64);

/// Rows a prepared selector indexed to build its discriminator.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct IndexBuildRows(pub u64);

/// Times a prepared selector took one selection path.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PathUses(pub u64);

/// Times a prepared selector's retention ended one admission way.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct AdmissionOutcomes(pub u64);

/// Diagnostic-only counts for one prepared selector in one file generation.
/// No row data or provider reference is retained here.
#[derive(Debug)]
pub(crate) struct SelectorHistogram {
    /// The prepared selector the counts belong to.
    pub selector: Arc<Selector>,
    /// Acquisitions of the selector.
    pub calls: SelectorCalls,
    /// Physical builds of its selection.
    pub builds: SelectionBuilds,
    /// Replays of its selection.
    pub reuses: SelectionReuses,
    /// Acquisitions that failed.
    pub errors: SelectorErrors,
    /// Rows it matched.
    pub matched_rows: MatchedRows,
    /// Rows visited by full scans.
    pub full_scan_rows: FullScanRows,
    /// Rows indexed to build its discriminator.
    pub index_build_rows: IndexBuildRows,
    /// Positions its discriminator admitted.
    pub indexed_candidates: AdmittedCandidates,
    /// Predicate cells it decoded.
    pub predicate_cells: DecodedPredicateCells,
    /// Uses of each selection path, by the path's place.
    pub paths: [PathUses; 9],
    /// Outcomes of each admission way, by the way's place.
    pub admission: [AdmissionOutcomes; 4],
}

impl From<&Arc<Selector>> for SelectorHistogram {
    fn from(selector: &Arc<Selector>) -> Self {
        Self {
            selector: Arc::clone(selector),
            calls: SelectorCalls::default(),
            builds: SelectionBuilds::default(),
            reuses: SelectionReuses::default(),
            errors: SelectorErrors::default(),
            matched_rows: MatchedRows::default(),
            full_scan_rows: FullScanRows::default(),
            index_build_rows: IndexBuildRows::default(),
            indexed_candidates: AdmittedCandidates::default(),
            predicate_cells: DecodedPredicateCells::default(),
            paths: [PathUses::default(); 9],
            admission: [AdmissionOutcomes::default(); 4],
        }
    }
}
