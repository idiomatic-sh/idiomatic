//! The types of the `walks` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_language::domain::relations::types::Value;

use crate::domain::kind_sets::types::KindScratch;
use crate::domain::operating::types::NameCount;
use crate::domain::table_cells::types::ColumnPlaces;
use crate::domain::table_scratch::types::{ComparisonNumbers, Marks, TextVerdicts};
use ryo_values::domain::plan_parts::types::Comparison;

/// Which elements of the tree are marked by position.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Marked(pub Vec<bool>);

/// How the elements a step reaches are told to be answers.
#[expect(
    clippy::large_enum_variant,
    reason = "an `Admit` lives on the stack for the one walk it decides and is never stored in a collection; boxing the filter would allocate once a walk"
)]
pub(crate) enum Admit<'x> {
    /// All of them.
    All,
    /// The elements marked by position.
    Marks(Marked),
    /// The elements the comparisons admit.
    Filter(FilterScratch<'x>, &'x [Comparison]),
}

/// How the entities a walk reaches are told to be answers: marked by place, when the domain is a table, or
/// decided where they are, when the domain is the elements of the tree and a filter of them.
pub(crate) enum Admission<'x> {
    /// The places of the entities the domain lists.
    Marked(&'x Marks),
    /// The elements the filter admits.
    Filtered(&'x [Comparison]),
}

/// What deciding an element by a filter works with, taken out of the scratch while the walk runs.
pub(crate) struct FilterScratch<'x> {
    /// The kind sets the comparisons test.
    pub kinds: KindScratch,
    /// The columns the comparisons the gate and kinds leave undecided read.
    pub needed: ColumnPlaces,
    /// The cells of the element being decided.
    pub cells: [Value; 5],
    /// How many names the symbols hold.
    pub names: NameCount,
    /// The comparisons of the filter.
    pub comparisons: &'x [Comparison],
    /// The comparisons that ask of the text of an element something fixed, decided once for each text.
    pub gate: ComparisonNumbers,
    /// What the gate decided for each text.
    pub text_ok: TextVerdicts,
    /// The comparisons the kind sets and the text gate have not decided.
    pub rest: ComparisonNumbers,
}
