//! One executable-program invocation owner; kernels never receive its inputs port.
pub(super) mod admission;
pub(super) mod runtime;
pub(super) mod kernel_session;
mod partitions;
pub(in crate::application) mod facade_runtime;
pub(crate) mod facade;

mod restored;

mod request_identity;
