//! One executable-program invocation owner; kernels never receive its inputs port.
pub(crate) mod admission;
pub(crate) mod facade_runtime;
pub(super) mod kernel_session;
pub(crate) mod preparation;
pub(crate) mod protocol;
pub(super) mod runtime;

pub(crate) mod restored;

mod request_identity;

mod context;
pub(in crate::application::file_dag) mod native_walk;

pub(in crate::application::file_dag) mod selection;

pub(in crate::application::file_dag) mod native_join;

mod project_population;

mod constants;
