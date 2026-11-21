//! The `tables` part of the lowering: one arm per operation. What a step's rows look like and whether it can be evaluated,
//! its branches, and the made tables of the operations that make one, each filled by pipelines of stages.

pub(crate) mod arms;
pub(crate) mod calls;
pub(crate) mod closures;
pub(crate) mod fills;
pub(crate) mod inputs;
pub(crate) mod joins;
pub(crate) mod matches;
pub(crate) mod selections;
pub(crate) mod service;
pub(crate) mod sets;
pub(crate) mod shapes;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
