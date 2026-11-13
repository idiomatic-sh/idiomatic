//! One executable-program invocation owner; kernels never receive its inputs port.
pub mod admission;
pub(crate) mod facade_runtime;
pub(crate) mod fact_lookup;
pub mod file_session;
pub(super) mod kernel_session;
pub mod observation_stage;
pub mod observed_publication;
pub mod preparation;
pub mod prepared_execution;
pub mod project_phase;
pub mod request_execution;
pub mod rule_outcome;
pub(super) mod runtime;

pub(crate) mod restored;

mod request_identity;

mod context;
pub(crate) mod native_walk;

mod project_population;

mod constants;
