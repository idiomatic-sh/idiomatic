//! The current interpreter's own `budget` counters of the project phase, read through the runtime's fixture accessors on
//! `ProjectPhase`. No case outside this group names them, and the rewrite deletes it whole.
#[cfg(test)]
mod fixtures;
#[cfg(test)]
pub(super) mod test;
