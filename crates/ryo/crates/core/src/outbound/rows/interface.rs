//! What the `rows` port asks of the outside: somewhere to write the answer as it is produced, so
//! nothing the core holds grows with the number of rows.

use super::errors::RowsError;
use super::types::{LentCell, Summary};
use ryo_values::domain::plan_parts::types::Heading;
use ryo_values::domain::records::types::Unread;

/// What every `rows` adapter implements.
pub trait RowsPort: Send + Sync {
    /// The headings of the columns, before the first row.
    fn begin(&self, columns: &[Heading]);

    /// One row, as its cells, typed and lent: a cell with no value is `None`. Whoever writes the row as text does so at the edge.
    fn row(&self, cells: &mut dyn Iterator<Item = LentCell<'_>>);

    /// Ends the answer with how many rows it has and the files nothing read.
    ///
    /// # Errors
    ///
    /// `Closed` when whoever reads the rows stopped; `Failed` when they could not be written.
    fn end(&self, summary: Summary, unread: &[Unread]) -> Result<(), RowsError>;
}
