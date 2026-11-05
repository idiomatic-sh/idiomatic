//! The types of the `hosted` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_language::domain::files::types::FilePath;

/// The files of a run in the order they are numbered: a file's number is its place.
#[derive(Clone, Copy, Debug)]
pub(crate) struct NumberedFiles<'a>(pub &'a [FilePath]);
