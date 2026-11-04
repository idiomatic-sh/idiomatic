//! One executable-program invocation owner; kernels never receive its inputs port.
pub(super) mod admission;
pub(crate) mod facade;
pub(in crate::application) mod facade_runtime;
pub(super) mod kernel_session;
mod partitions;
pub(super) mod runtime;

mod restored;

mod request_identity;

mod context;
pub(in crate::application::file_dag) mod native_walk;

pub(in crate::application::file_dag) mod selection;
