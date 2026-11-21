//! The `planning` part of the lowering: a branch's accesses ordered by the cost model, then written by one of two emitters:
//! steps for a rule's branch or a call-form pattern, stages for a table's pipeline.

pub(crate) mod closures;
pub(crate) mod completing;
pub(crate) mod deciding;
pub(crate) mod deferring;
pub(crate) mod emitting;
pub(crate) mod forms;
pub(crate) mod operands;
pub(crate) mod service;
pub(crate) mod types;
pub(crate) mod uses;
pub(crate) mod walks;
pub(crate) mod writing;

#[cfg(test)]
mod service_test;
