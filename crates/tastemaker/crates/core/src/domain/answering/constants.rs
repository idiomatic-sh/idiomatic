//! The names of the `answering` concept: the columns of the rows the lint call answers that the comparison and the
//! imports read.

/// A row's span.
pub(crate) const SPAN: &str = tastemaker_values::domain::import_rows::constants::SPAN;
/// The directory a test lives in.
pub(crate) const TESTS: &str = tastemaker_values::domain::imports::constants::TESTS;
/// A comment row's column that says whether the comment is documentation.
pub(crate) const DOC: &str = "doc";
/// A comment row's column that holds its text.
pub(crate) const TEXT: &str = "text";
/// How a raised flag cell is written: a comment row's `doc` when the comment is documentation.
pub(crate) const RAISED: &str = "true";
