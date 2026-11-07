//! The types of the `describe` use case: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_values::domain::descriptions::types::Grammar;

/// The grammar of RyoQL.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DescribeReport {
    /// Every production, from the whole program down to a word.
    pub grammar: Grammar,
}
