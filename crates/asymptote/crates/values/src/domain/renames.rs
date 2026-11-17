//! The `renames` concept: carrying accepted records through git's renames, so moving a file keeps its acceptances.
//! A rename carries a record to the new path only when the run reports its identity there; otherwise `down` drops it.

pub mod constants;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
