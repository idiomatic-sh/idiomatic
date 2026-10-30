//! The types of the `table_words` concept: plain data, public fields and derives, with no
//! constructor and no inherent impl. A table is written as words and read back from them.

/// Words a table was written as, borrowed.
#[derive(Clone, Copy, Debug)]
pub(crate) struct WordSlice<'a>(pub &'a [u64]);
