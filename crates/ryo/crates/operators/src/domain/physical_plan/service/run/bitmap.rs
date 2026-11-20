//! Spike (ryo-bshs): the existence pre-pass. Before a node is made, each plan node is evaluated over key sets only: per column
//! a bitset over the open file's dense ids (a node's position, a symbol's number, any other cell hashed), never rows. A node
//! whose answer is proved to hold no row is only marked empty.
pub(crate) mod service;
