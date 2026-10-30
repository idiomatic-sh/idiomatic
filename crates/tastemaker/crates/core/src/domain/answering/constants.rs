//! The names of the `answering` concept: the columns of the rows the lint call answers that the comparison and the
//! imports read.

/// A row's span.
pub const SPAN: &str = crate::domain::import_rows::constants::SPAN;
/// The directory a test lives in.
pub const TESTS: &str = crate::domain::imports::constants::TESTS;
/// A comment row's column that says whether the comment is documentation.
pub const DOC: &str = "doc";
/// A comment row's column that holds its text.
pub const TEXT: &str = "text";
/// How a raised flag cell is written: a comment row's `doc` when the comment is documentation.
pub const RAISED: &str = "true";
