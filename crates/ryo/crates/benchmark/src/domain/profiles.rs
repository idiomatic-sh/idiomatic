//! A profile run's pure values: the heap dumps in their order, the peak, the CPU sampling rate and the heap profiler's configuration.
pub(crate) mod service;
#[cfg(test)]
mod service_test;
pub mod types;
