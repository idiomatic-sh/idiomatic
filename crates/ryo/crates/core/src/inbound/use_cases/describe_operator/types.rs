//! The types of the `describe_operator` use case: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_values::domain::descriptions::types::OperatorDescription;
use ryo_values::domain::descriptions::types::OperatorName;

/// The operator to describe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DescribeOperatorRequest {
    /// Its name: `join`, `group-count`.
    pub name: OperatorName,
}

/// One operator, described.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DescribeOperatorReport {
    /// The operator asked for, and no other.
    pub operator: OperatorDescription,
}
