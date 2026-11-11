//! Opt-in diagnostic accounting; never participates in execution or admission decisions.
mod constants;
#[cfg(test)]
mod fixtures;
pub mod ids;
pub mod profile;
#[cfg(feature = "diagnostic-row-progress")]
pub mod progress;
mod record;
pub(crate) mod service;
#[cfg(test)]
mod service_test;
pub mod types;
