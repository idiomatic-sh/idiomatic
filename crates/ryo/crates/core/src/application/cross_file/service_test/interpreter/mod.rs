//! The current interpreter's own counters of the project phase, read through the runtime's fixture accessors on `ProjectPhase`.
//! No case outside this directory names them, and the rewrite deletes it whole.
#[cfg(test)]
mod budget;
#[cfg(test)]
mod population;
#[cfg(test)]
mod root_set;
