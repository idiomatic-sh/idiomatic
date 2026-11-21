//! The types of the `sharing` part.

use crate::domain::steps::types::MadeTableId;

/// A table the sharing made: the rows of a run of stages several pipelines of one table began with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SharedPrefix {
    /// The table of the prefix's rows.
    pub(crate) table: MadeTableId,
    /// The table whose pipelines read it.
    pub(crate) of: MadeTableId,
}
