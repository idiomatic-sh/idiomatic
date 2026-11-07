//! The types of the `predicate` concept: plain data, public fields and derives, with no constructor and no inherent impl.
use crate::domain::sharing::types::SharedPositions;
use ryo_values::domain::plan_parts::types::LiteralText;

/// The rows whose symbol cell stands for one literal text.
#[derive(Debug)]
pub struct LiteralPostings {
    /// The literal text.
    pub text: LiteralText,
    /// The positions of the rows that have it.
    pub rows: SharedPositions,
}
