//! One executable-program invocation owner; kernels never receive its inputs port.
pub mod admission;
pub(crate) mod facade_runtime;
pub mod file_session;
pub mod observation_stage;
pub mod observed_publication;
pub mod preparation;
pub mod prepared_execution;
pub mod project_phase;
pub mod request_execution;
pub mod rule_outcome;


pub(crate) mod request_identity;



pub(crate) mod constants;
