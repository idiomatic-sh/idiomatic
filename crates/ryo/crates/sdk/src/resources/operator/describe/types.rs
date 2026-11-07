//! The types of `operator describe`: what is asked and what comes back.

use crate::domain::operator::types::{Example, OperatorName, Purpose};

/// The name of a field of an operator.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FieldName(pub Box<str>);

/// One closed choice an operator offers, one line.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Choice(pub Box<str>);

/// The operator to describe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DescribeRequest {
    /// Its name: `join`, `group-count`.
    pub name: OperatorName,
}

/// One operator, described.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct DescribeReport {
    /// The operator asked for, and no other.
    pub operator: OperatorDescription,
}

/// What an operator is, what it takes and an example.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct OperatorDescription {
    /// The name a query writes.
    pub name: OperatorName,
    /// What it does, in a sentence.
    pub summary: Purpose,
    /// The fields it takes.
    pub fields: Vec<FieldDescription>,
    /// The closed choices it offers, one line each, for an operator that has any.
    pub options: Vec<Choice>,
    /// The operator written as one statement of query text.
    pub example: Example,
}

/// Whether an operator cannot be written without a field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Requirement {
    /// The operator cannot be written without it.
    Required,
    /// The operator can be written without it.
    Optional,
}

/// One field of an operator.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct FieldDescription {
    /// The field's name.
    pub name: FieldName,
    /// What it is, in a phrase.
    pub summary: Purpose,
    /// Whether the operator cannot be written without it.
    pub required: Requirement,
}
