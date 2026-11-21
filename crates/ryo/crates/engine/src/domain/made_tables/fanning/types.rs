//! The types of the `fanning` part: what a pipeline's scan reads.

use crate::domain::code::types::IndexSite;
use crate::domain::indexes::types::RowRun;
use crate::domain::made_tables::types::ReadSource;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

/// What a pipeline's scan reads: its source; of a bucket the site and the key's stretch of that site's index; how many
/// rows; the first row of a whole source; and the bits every handle carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ScanRead {
    /// The source.
    pub(crate) source: ReadSource,
    /// Of a bucket: the site whose index holds it.
    pub(crate) site: Option<IndexSite>,
    /// Of a bucket: the key's stretch of the index's rows. Of a whole source: its rows.
    pub(crate) rows: RowRun,
    /// How many rows the scan reads.
    pub(crate) count: Count,
    /// The bits every handle carries besides the row's number.
    pub(crate) tag: CellWord,
}
