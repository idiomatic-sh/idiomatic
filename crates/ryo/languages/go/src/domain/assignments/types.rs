//! The types of the `assignments` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::tokens::types::Token;
use ryo::languages::Position;

/// The operator of an assignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Operator {
    /// Its token, among the tokens of the element.
    pub at: Position,
    /// Which operator it is.
    pub token: Token,
}
