//! The `seeding` concept: the perf records `up` writes from the last check's medians, a first record for a median with
//! none and a raised best for one over its ceiling.

pub mod service;

#[cfg(test)]
mod service_test;
