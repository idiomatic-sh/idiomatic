//! Typed source selections and their exact ordered predicate equivalence classes.

use crate::domain::coverage::types::RuleIndex;
use crate::domain::operator::types::StepPlace;
use ryo_values::domain::plan_parts::types::{ColumnIndex, PlanComparison};

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
    /// The partition whose rows the class selects: a class never spans two partitions.
    pub scope: ryo_values::domain::plan_parts::types::StepScope,
    /// The step the class is decided by.
    pub first: MergedStep,
    /// Every step of the class, the first included.
    pub members: Vec<MergedStep>,
    /// Ordered predicates mapped through preceding selections to source columns.
    pub comparisons: Vec<PlanComparison>,
}

/// A selection chain mapped to its original typed relation and source columns.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Rooted {
    /// The original relation, before any projection.
    pub source: ryo_relations::domain::relations::types::Relation,
    /// The comparisons over the source's columns.
    pub comparisons: Vec<PlanComparison>,
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

/// Whether two requests bind equal typed values wherever a predicate reads them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BindingsAgreement {
    Equal,
    Differ,
}

/// A class of the steps that select from the elements of a tree, by its place among the classes of the call's rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ClassPlace(pub usize);
