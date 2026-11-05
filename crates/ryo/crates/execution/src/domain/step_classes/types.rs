//! Typed source selections and their exact ordered predicate equivalence classes.

use crate::domain::coverage::types::RuleIndex;
use crate::domain::operator::types::StepPlace;
use crate::domain::positions::types::ClassPlace;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Comparison};

/// A per-file selection's rule and place among that rule's steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MergedStep {
    /// The place of the rule among the rules of the call.
    pub rule: RuleIndex,
    /// The place of the step among the rule's steps.
    pub step: StepPlace,
}

/// Steps over one relation with equal ordered comparisons and referenced typed bindings.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct StepClass {
    /// The typed relation whose ordered rows these comparisons select.
    pub source: ryo_relations::domain::relations::types::Relation,
    /// The step the class is decided by.
    pub first: MergedStep,
    /// Every step of the class, the first included.
    pub members: Vec<MergedStep>,
    /// Ordered predicates mapped through preceding selections to source columns.
    pub comparisons: Vec<Comparison>,
}

/// A selection chain mapped to its original typed relation and source columns.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Rooted {
    /// The original relation, before any projection.
    pub source: ryo_relations::domain::relations::types::Relation,
    /// The comparisons over the source's columns.
    pub comparisons: Vec<Comparison>,
    /// For each output column, the source column it reads.
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
