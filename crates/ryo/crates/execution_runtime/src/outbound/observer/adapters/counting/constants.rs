//! The constants of the counting adapter.

/// How many rules the counting observer keeps totals for: a rule at or past it is not counted, so what it keeps stays bounded.
pub(crate) const COUNTED_FED: usize = 1 << 16;
