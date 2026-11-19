//! The runner's one row loop: the chain of streamed stages that feeds a node, its rows pushed stage by stage in bounded chunks,
//! only the node's own rows held.

pub(crate) mod push;
pub(crate) mod service;
pub(crate) mod sources;
pub(crate) mod stages;
// Spike (ryo-vhuy): typed chains.
pub(crate) mod typed;
