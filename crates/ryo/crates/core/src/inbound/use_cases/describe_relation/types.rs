//! The types of the `describe_relation` use case: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_values::domain::descriptions::types::RelationDescription;
use ryo_values::domain::descriptions::types::RelationName;

/// The relation to describe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DescribeRelationRequest {
    /// Its name.
    pub name: RelationName,
}

/// One relation, described.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DescribeRelationReport {
    /// The relation asked for, and no other.
    pub relation: RelationDescription,
}
