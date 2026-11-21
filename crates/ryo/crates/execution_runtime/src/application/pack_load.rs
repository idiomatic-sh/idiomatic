//! The `pack_load` operation (scope-engine2 2.5 and 2.7, track H): the rule set of a run, read back from the store's pack under
//! the digest of what the run was given, the kind vocabulary and the engine, with no plan in hand, or lowered from the compiled
//! rules and written for the store. On the build branch it starts the `RYO_ENGINE` harness: what the new engine's `load`
//! accepts, by count.

pub mod constants;
pub(crate) mod selecting;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
