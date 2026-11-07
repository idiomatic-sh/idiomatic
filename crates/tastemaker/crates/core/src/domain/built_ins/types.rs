//! The types of the `built_ins` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::applications::types::GlobText;
use crate::domain::contracts::types::{ComparisonQuery, Syntax};

/// One built-in rule of the comparison: what it reads, its query, and the files it reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Comparison {
    /// What it reads.
    pub syntax: Syntax,
    /// Its query.
    pub query: ComparisonQuery,
    /// The files it reads.
    pub include: Vec<GlobText>,
}
