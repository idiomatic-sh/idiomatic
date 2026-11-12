//! The runtime's one reach into the store's derived owner: it prepares a request's program over the derived access,
//! acquires the views an invocation binds and the file's cached graphs, and lends each only as its typed view.
pub(crate) mod axis_resources;
pub(crate) mod invocation;
pub(crate) mod program;
pub mod service;
