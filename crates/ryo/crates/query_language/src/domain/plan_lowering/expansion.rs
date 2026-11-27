//! The `expansion` pass (`lower6d.py`): every rule's program lowered to steps, the fragments it imports inlined with the
//! caller's inputs and parameters, each step shared by every rule that lowers to an equal one (the first to make it
//! numbers it). A rule that does not lower keeps the steps it made before it stopped.

pub(crate) mod service;
pub(crate) mod types;

