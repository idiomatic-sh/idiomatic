//! The names of the `idioms` concept.

use ryo_values::domain::plan_parts::types::ColumnIndex;

/// The column of the digests' step that holds the function whose texts it reduced: the step's one key, before its digest and size.
pub(crate) const DIGEST_OWNER: ColumnIndex = ColumnIndex(0);
