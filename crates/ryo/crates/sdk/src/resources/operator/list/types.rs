//! The types of `operator list`: what is asked and what comes back.

use crate::domain::operator::types::{OperatorName, Purpose};

/// Every operator, in documentation order.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ListReport {
    /// The operators.
    pub operators: Vec<OperatorSummary>,
}

/// One line about an operator.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct OperatorSummary {
    /// The name a query writes.
    pub name: OperatorName,
    /// What it does, in a sentence.
    pub summary: Purpose,
}
