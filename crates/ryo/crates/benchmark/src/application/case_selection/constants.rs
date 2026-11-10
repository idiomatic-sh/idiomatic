//! Refusals of a case selection.
/// A selection that names no case, or a case outside the shard, is refused.
pub(super) const CASE_SELECTION_IS_EMPTY_OR_OUTSIDE: &str =
    "case selection is empty or outside this shard";
