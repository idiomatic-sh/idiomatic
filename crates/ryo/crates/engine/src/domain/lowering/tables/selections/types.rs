//! The types of the `selections` part.

use crate::domain::lowering::nodes::types::StepShape;
use crate::domain::lowering::tables::sets::types::SetTests;
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::plan_parts::types::{PlanComparison, Side};

/// The comparisons of a selection, with what they are written over: the shape of the row they read (a comparison names its
/// columns), and how a set of literals is taken.
#[derive(Clone, Debug)]
pub(crate) struct SelectionAsked<'a> {
    /// The comparisons.
    pub(crate) comparisons: &'a [PlanComparison],
    /// What each column of the row they read holds.
    pub(crate) shape: StepShape,
    /// How a kind or a field held to a set of literals is taken.
    pub(crate) sets: SetTests,
}

/// A text a comparison holds a flag column to: which side of the comparison the text is on, and the flag's own cell when
/// the text is a word a flag is shown as (`true`, `false`); none for any other text, which no flag equals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FlagWord {
    /// The side the text is on.
    pub(crate) side: Side,
    /// The cell of the flag the text shows.
    pub(crate) cell: Option<Integer>,
}
