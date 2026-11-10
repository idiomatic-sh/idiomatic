//! What one primitive evaluation is given: its prepared inputs.
use crate::domain::operating::types::InputPlace;
use crate::domain::table_model::types::RepresentationInput;
use ryo_relations::domain::relations::types::Value;

/// Immutable inputs only. No Feed, callback, scheduler or owned/retained Arc is supplied.
#[derive(Clone, Copy, Debug)]
pub enum PreparedInputs<'a> {
    /// One row of the triggering input with its immutable side inputs.
    Row {
        /// The input the row comes from.
        input: InputPlace,
        /// The row's cells.
        row: &'a [Value],
        /// The side inputs the row is evaluated against.
        sides: &'a [RepresentationInput<'a>],
    },
    /// The whole inputs, each as one representation.
    Views(&'a [RepresentationInput<'a>]),
    /// The source the request supplies.
    RequestSource(&'a RepresentationInput<'a>),
}
