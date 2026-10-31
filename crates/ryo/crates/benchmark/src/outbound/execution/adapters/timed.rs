//! Direct per-child Linux resource observations.
pub(crate) mod adapter;
#[cfg(target_os = "linux")]
mod schemas;

#[cfg(test)]
mod adapter_test;

#[cfg(target_os = "linux")]
mod constants;
