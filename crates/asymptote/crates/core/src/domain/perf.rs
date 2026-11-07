//! The `perf` concept: what a tool measured, judged against the best the lock recorded for it. A measured median above
//! the best fails; one below it rewrites the best.

pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
