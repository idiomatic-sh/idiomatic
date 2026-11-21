//! The types of the `found_rows` concept: plain data, public fields and derives, with no constructor and no inherent impl. What
//! the rows a rule found in a file are rendered with, and where two renderings of one file differ.

use crate::domain::render::types::{RowLayout, RowLimit, Shown};
use crate::domain::value_cells::types::Scene;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::query_plans::types::Emit;
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

/// How the rows of one rule are compared between the two engines' renderings of a file: which cells a row shows, what the rule
/// emits, and what each of its columns holds. A cell that holds a node is not compared: the engines number nodes differently.
#[derive(Clone, Copy)]
pub(crate) struct Compared<'a> {
    /// Which cells a row of the rule shows.
    pub shown: Shown<'a>,
    /// What a row of the rule emits.
    pub emit: &'a Emit,
    /// What each column of the rule's rows holds, in column order.
    pub kinds: &'a [CellKind],
}

/// Whether a cell of a rendered row is compared between the two engines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CellCompared {
    /// It is: a file, a text, a number or a span.
    Compared,
    /// It is not: it holds a node, which the engines number differently.
    Node,
}

/// A rendered row as the text it is compared by: its cells joined by tabs, a node's cell left out.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ComparedRow(pub Box<str>);

/// The first row at which the two engines' renderings of one file differ, for the first rule whose rows do: the row each engine
/// has there, none for the engine that has no more rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RowDifference {
    /// The rule.
    pub rule: RuleNumber,
    /// The old engine's row.
    pub old: Option<ComparedRow>,
    /// The new engine's row.
    pub new: Option<ComparedRow>,
}

/// SPIKE (ryo-fmx3.1): the new engine's rows of the rules whose rows today's engine makes after the files, as the text they
/// are compared by, by rule, gathered over the files a run compared; and the rule set whose language walked them.
#[derive(Debug, Default)]
pub(crate) struct LaterRows {
    /// The rows, by rule.
    pub rows: std::collections::BTreeMap<RuleNumber, Vec<ComparedRow>>,
    /// The rule set that ran them, when one did.
    pub by: Option<ryo_execution::inbound::use_cases::file_scan::types::RuleSetNumber>,
}
