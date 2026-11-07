//! The types of the `variables` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use std::collections::BTreeSet;

use crate::domain::entries::types::Variables;
use crate::domain::selectors::types::VariableName;

/// What is wrong with a file's variables.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum VariableProblem {
    /// A `{ variable = … }` names no variable of its file.
    Undefined(VariableName),
    /// A variable no entry refers to.
    Unused(VariableName),
    /// A variable spliced into a selector list holds something that is not a selector.
    NotSelectors(VariableName),
}

/// What is being resolved: the variables and which of them were read.
#[derive(Debug)]
pub struct Resolving<'a> {
    /// The variables of the file.
    pub variables: &'a Variables,
    /// The variables read so far.
    pub used: BTreeSet<VariableName>,
    /// What is wrong so far.
    pub problems: BTreeSet<VariableProblem>,
}
