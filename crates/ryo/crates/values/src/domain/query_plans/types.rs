//! The types of the `query_plans` concept: what a query emits (a stored pack still holds it). Plain data, public fields and derives, with no
//! constructor and no inherent impl.

use crate::domain::plan_parts::types::ColumnIndex;
use crate::domain::plan_parts::types::Heading;
use crate::domain::plan_parts::types::StepId;

/// What the query emits.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::plan_parts::types::{ColumnIndex, Heading, StepId};
/// use ryo_values::domain::query_plans::types::Emit;
/// let emit = Emit {
///     step: StepId(2),
///     subject: ColumnIndex(0),
///     key: vec![ColumnIndex(1)],
///     headings: vec![Heading("file".into()), Heading("name".into())],
/// };
/// assert_eq!(emit.key, [ColumnIndex(1)]);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Emit {
    /// The step whose rows answer the query.
    pub step: StepId,
    /// The column holding the file a row is about.
    pub subject: ColumnIndex,
    /// The columns that identify a row.
    pub key: Vec<ColumnIndex>,
    /// The headings of the answer's columns: text that leaves the engine.
    pub headings: Vec<Heading>,
}
