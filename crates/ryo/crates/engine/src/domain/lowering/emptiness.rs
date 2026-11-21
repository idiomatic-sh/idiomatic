//! The `emptiness` part of the lowering: what a file must hold for a step to have a row, one formula per operation.

pub(crate) mod needs;
pub(crate) mod service;

#[cfg(test)]
mod service_test;
