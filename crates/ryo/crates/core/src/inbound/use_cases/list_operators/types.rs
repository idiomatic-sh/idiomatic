//! The types of the `list_operators` use case: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_values::domain::descriptions::types::{OperatorName, Purpose};

/// Every operator, in documentation order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListOperatorsReport {
    /// The operators.
    pub operators: Vec<OperatorSummary>,
}

/// One line about an operator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperatorSummary {
    /// The name a query writes.
    pub name: OperatorName,
    /// What it does, in a sentence.
    pub summary: Purpose,
}
