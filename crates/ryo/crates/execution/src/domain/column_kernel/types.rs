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

/// Actual successful coverage; cells count explicit physical-cell decodes by the kernel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct KernelCounts {
    pub compiled_chains: u64,
    pub eligible_chains: u64,
    pub covered_rows: u64,
    pub decoded_cells: u64,
    pub joined_hits: u64,
    /// Rows passing the total filter before terminal distinct projection.
    pub filtered_rows: u64,
}
