//! The runtime's one reach into the store's derived owner: it holds the request's access, acquires the views an
//! invocation binds and the file's cached graphs, and lends each only as its typed view.
pub(crate) mod graph;
pub mod service;
pub(crate) mod view;
