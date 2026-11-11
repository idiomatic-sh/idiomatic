//! The current interpreter's own tests: what it counts while it runs, read through the runtime's fixture accessors. No test
//! outside this directory names it, and the rewrite deletes it whole.
mod activation;
mod file_intents;
#[cfg(test)]
pub(crate) mod fixtures;
mod project_phase;
mod publication;
mod scheduler;
mod shared_walk;
