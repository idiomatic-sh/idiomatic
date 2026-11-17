//! The test support of the `entries` concept: a value writer for the cases that pass no param.

use crate::domain::entries::types::{Value, ValueText};

/// A value writer that is never asked: the cases it is handed to pass no param.
///
/// # Panics
///
/// When it is asked to write `value`, which a case that passes no param never does.
#[must_use]
pub fn unwritten(value: &Value) -> ValueText {
    panic!("nothing passes a param, but {value:?} was written")
}
