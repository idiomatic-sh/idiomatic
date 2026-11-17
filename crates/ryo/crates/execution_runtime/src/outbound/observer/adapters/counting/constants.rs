//! The constants of the counting adapter.

/// How many rules the counting observer keeps totals for: a rule at or past it is not counted, so what it keeps stays bounded.
pub(crate) const COUNTED_FED: usize = 1 << 16;

/// How many handovers the counting observer keeps: one for each file and rule that handed the steps after the files anything, so a
/// run over a few thousand files with a few hundred rules fits; one past it is refused, so what it keeps stays bounded.
pub(crate) const COUNTED_HANDOVERS: usize = 1 << 21;
