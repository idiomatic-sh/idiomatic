//! The types of the `selections` part.

use crate::domain::lowering::nodes::types::StepShape;
use crate::domain::lowering::tables::sets::types::SetTests;
use ryo_values::domain::plan_parts::types::{Input, PlanComparison};

/// The comparisons of a selection, with what they are written over: the shape of the row they read (a comparison names its
/// columns), and how a set of literals is taken.
#[derive(Clone, Debug)]
pub(crate) struct SelectionAsked<'a> {
    /// The comparisons.
    pub(crate) comparisons: &'a [PlanComparison],
    /// What they read the rows of.
    pub(crate) read: Input,
    /// What each column of the row they read holds.
    pub(crate) shape: StepShape,
    /// How a kind or a field held to a set of literals is taken.
    pub(crate) sets: SetTests,
}
