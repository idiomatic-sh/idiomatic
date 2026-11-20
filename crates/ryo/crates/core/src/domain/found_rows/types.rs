//! The types of the `found_rows` concept: plain data, public fields and derives, with no constructor and no inherent impl. What
//! the rows a rule found in a file are rendered with.

use crate::domain::render::types::{RowLayout, RowLimit};
use crate::domain::value_cells::types::Scene;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plans::types::Emit;
use ryo_values::domain::runs::types::RuleNumber;

/// What one call renders of the rows the new engine's scan kept: the rule that found them, what a row of it emits, what each of
/// its columns holds, the cells shown of it, the file they are of, and how many rows may be made into cells.
#[derive(Clone, Copy)]
pub(crate) struct FoundInput<'a> {
    /// The rule that found the rows.
    pub rule: RuleNumber,
    /// What a row of the rule emits: its subject, its key and its headings.
    pub emit: &'a Emit,
    /// What each column of the rule's rows holds, in column order.
    pub kinds: &'a [CellKind],
    /// The cells shown of a row.
    pub layout: RowLayout<'a>,
    /// The file the rows are of.
    pub scene: Scene<'a>,
    /// How many rows are made into cells; the rest are counted.
    pub limit: RowLimit,
}
