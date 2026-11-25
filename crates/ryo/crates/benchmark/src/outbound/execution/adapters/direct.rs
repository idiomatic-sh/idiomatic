//! Direct per-child Linux resource observations.
pub(crate) mod adapter;
#[cfg(target_os = "linux")]
mod schemas;


#[cfg(target_os = "linux")]
mod constants;
