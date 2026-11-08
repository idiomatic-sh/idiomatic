//! One executable-program invocation owner; kernels never receive its inputs port.
pub mod admission;
pub(crate) mod facade_runtime;
pub mod file_session;
pub mod observation_stage;
pub mod prepared_execution;
pub mod project_phase;
pub mod request_execution;
pub(super) mod kernel_session;
pub mod preparation;
pub mod observed_publication;
pub mod rule_outcome;
pub(super) mod runtime;

pub(crate) mod restored;

mod request_identity;
#[cfg(test)]
mod request_identity_test;

mod context;
pub(crate) mod native_walk;


mod project_population;

mod constants;
