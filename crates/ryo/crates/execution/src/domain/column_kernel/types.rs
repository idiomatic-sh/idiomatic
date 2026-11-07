//! Immutable kernel programs and request-owned execution counters.

use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{ColumnIndex, JoinColumn, LiteralText};
use std::sync::Arc;

use crate::domain::derived_views::types::DerivedContract;

/// One final kept cut whose intermediate join and equality filter can stay as row IDs.
#[derive(Clone, Debug)]
pub(crate) struct Chain {
    /// Existing kept compact cut; this chain never re-evaluates its producer.
    pub left_step: usize,
    /// The equality filter and the join the chain replaces, by their step numbers in the plan.
    pub filter_step: ryo_values::domain::plan_parts::types::StepId,
    pub join_step: ryo_values::domain::plan_parts::types::StepId,
    /// Complete rectangular relation looked up by typed equality.
    pub right: Relation,
    pub left_on: Vec<ColumnIndex>,
    pub right_on: Vec<ColumnIndex>,
    /// One total Symbol equality against an authored nonempty text literal.
    pub filter: JoinColumn,
    pub literal: LiteralText,
    /// Final output columns mapped directly to their source side.
    pub projected: Vec<JoinColumn>,
    /// Prepared once by the application; the domain shape compiler does no registration.
    pub index: Option<Arc<DerivedContract>>,
}

/// One prepared plan's optional chains, indexed by final step number.
#[derive(Clone, Debug, Default)]
pub(crate) struct KernelPlan(pub Vec<Option<Chain>>);

/// How many chains the kernel found eligible and ran.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ChainsRun(pub u64);

/// How many left rows a run walked.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RowsCovered(pub u64);

/// How many physical cells the kernel decoded explicitly.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CellsDecoded(pub u64);

/// How many right rows matched a left key.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct JoinedHits(pub u64);

/// How many rows passed the total filter before terminal distinct projection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RowsFiltered(pub u64);

/// Actual successful coverage; cells count explicit physical-cell decodes by the kernel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct KernelCounts {
    pub eligible_chains: ChainsRun,
    pub covered_rows: RowsCovered,
    pub decoded_cells: CellsDecoded,
    pub joined_hits: JoinedHits,
    /// Rows passing the total filter before terminal distinct projection.
    pub filtered_rows: RowsFiltered,
}

/// What one successful run of a chain measured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct KernelMeasure {
    pub emitted: RowsFiltered,
    pub decoded: CellsDecoded,
    pub hits: JoinedHits,
}

/// Whether a cell is a Symbol that equals the authored literal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SymbolVerdict {
    Holds,
    Differs,
}

/// The column and nonempty authored text of a total Symbol equality filter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SymbolFilter {
    pub column: ColumnIndex,
    pub literal: LiteralText,
}
