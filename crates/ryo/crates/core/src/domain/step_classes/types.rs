//! The types of the `step_classes` concept: the steps of the call's rules that select from the elements of a file, and the classes of
//! those that select the same elements. Plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::coverage::types::RuleIndex;
use crate::domain::operator::types::StepPlace;
use crate::domain::positions::types::ClassPlace;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Comparison};

/// A step that selects from the elements of a file: the rule it is a step of, and its place among the rule's steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MergedStep {
    /// The place of the rule among the rules of the call.
    pub rule: RuleIndex,
    /// The place of the step among the rule's steps.
    pub step: StepPlace,
}

/// The steps that select the same elements: they have the same comparisons, under the same bindings of their rules' parameters,
/// so what one decides of an element or a text the others have decided too.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct StepClass {
    /// The step the class is decided by.
    pub first: MergedStep,
    /// Every step of the class, the first included.
    pub members: Vec<MergedStep>,
    /// The comparisons that select its elements from the tree: those of the step, and, when it selects from a step that selects from
    /// the tree, those of that step before them, over the tree's own columns.
    pub comparisons: Vec<Comparison>,
}

/// A step that keeps elements of the tree: the comparisons that select them from the tree, and which column of the tree each column
/// of its rows is.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Rooted {
    /// The comparisons over the tree's columns.
    pub comparisons: Vec<Comparison>,
    /// For each column of the step's rows, the column of the tree it is.
    pub columns: Vec<ColumnIndex>,
}

/// The classes the steps of the call's rules make, and the class each step is in.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Merge {
    /// The classes, in the order their first step appears among the rules.
    pub classes: Vec<StepClass>,
    /// For each rule and each of its steps, the class that selects it, when it is one the merge covers.
    pub class_of: Vec<Vec<Option<ClassPlace>>>,
}
