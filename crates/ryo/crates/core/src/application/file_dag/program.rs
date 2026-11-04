//! One executable-program invocation owner; kernels never receive its inputs port.
pub(super) mod admission;
pub(super) mod runtime;
mod partitions;
pub(in crate::application) mod facade_runtime;
pub(crate) mod facade;
