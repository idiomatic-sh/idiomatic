//! The types of the `fact_lookup` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_execution_domain::domain::coverage::types::Recovery;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::stored_facts::types::ReadInPart;

/// What a file is read for: the relations its sources are asked for, those admitted, and core's own said parts and recovery.
#[derive(Clone, Copy, Debug)]
pub(crate) struct FileWants<'a> {
    /// What the sources are asked for.
    pub(crate) wanted: &'a [Relation],
    /// What is admitted and counts towards partiality.
    pub(crate) active: &'a [Relation],
    /// What core said is read in part of its own rows: the engine's rows and a failure's.
    pub(crate) said: &'a [ReadInPart],
    /// Whether what was read is the whole file or a recovery.
    pub(crate) recovery: Recovery,
}

/// Whether a file's rows were read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RowsRead {
    /// Not yet, or they were released: nothing is readable.
    Closed,
    /// The demanded relations were read and the active ones among them admitted.
    Read,
}
