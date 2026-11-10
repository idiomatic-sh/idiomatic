//! The constants of the memory adapter.

/// What a refusal says when a worker's columns are not one for each relation of the catalog.
pub(crate) const SCHEMAS: &str = "the columns are not one for each relation";

/// How many words of room a column keeps when its worker is released; the rest goes back, and a larger file grows it again.
pub(crate) const RETAINED_COLUMN_WORDS: usize = 1 << 13;
