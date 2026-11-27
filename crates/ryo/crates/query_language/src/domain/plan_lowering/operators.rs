//! The `operators` part: an operator's call, its arguments resolved (a relation to its step, every other argument to a
//! constant), as the plan's typed step of that operator; an operator outside the step set is kept as written.

pub(crate) mod service;

#[cfg(test)]
mod vocabulary_test;
