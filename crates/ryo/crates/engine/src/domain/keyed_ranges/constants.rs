//! The constants of the `keyed_ranges` concept.

/// The start a key is given when it is asked and its rows are not filed yet.
pub(crate) const PENDING: u32 = u32::MAX;

/// A slot keeps its keys by position in the next file when, in the last file it was used in, it held at least this many
/// keys and at least a `DENSE_SHARE`-th of that file's dense bound: a position array of a file's size then costs no more
/// than the flat hash it replaces.
pub(crate) const DENSE_MIN_KEYS: u32 = 256;
/// See `DENSE_MIN_KEYS`.
pub(crate) const DENSE_SHARE: u64 = 4;
