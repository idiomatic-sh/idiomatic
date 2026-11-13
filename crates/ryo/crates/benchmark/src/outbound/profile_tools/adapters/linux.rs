//! The Linux profilers: `jeprof`, `perf` and `c++filt`.
pub(crate) mod adapter;
#[cfg(test)]
mod adapter_test;

mod constants;
mod schemas;
