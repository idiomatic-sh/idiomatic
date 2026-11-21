//! The types of the `built` part: a part of the code the fill side is asked about.

use crate::domain::code::types::{CodeWord, FillCode};

/// A part of the code the fill side is asked whether it runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FillPart {
    /// A stage of a pipeline, by its word.
    Stage(CodeWord),
    /// A made table's fill, by its code.
    Fill(FillCode),
}
