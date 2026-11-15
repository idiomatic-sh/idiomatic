//! The runner's one row loop: the chain of streamed stages that feeds a node, its rows pushed stage by stage in bounded chunks,
//! only the node's own rows held.

pub(crate) mod service;
#[cfg(test)]
mod service_test;
