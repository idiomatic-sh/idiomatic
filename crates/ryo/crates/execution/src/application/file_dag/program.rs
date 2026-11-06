//! One executable-program invocation owner; kernels never receive its inputs port.
pub(crate) mod admission;
pub(crate) mod facade;
pub(crate) mod facade_runtime;
pub(super) mod kernel_session;
pub(super) mod runtime;

mod restored;

mod request_identity;

mod context;
pub(in crate::application::file_dag) mod native_walk;

pub(in crate::application::file_dag) mod selection;

pub(in crate::application::file_dag) mod native_join;

mod project_population;

mod constants;
