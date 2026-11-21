//! The `deferring` part of the planning: a read of a column waits until its cell is first needed.

pub(crate) mod service;
pub(crate) mod types;
