//! The `flow` part of the model: the schedule interpreted over sparse namespaces. It states what the language does with each
//! name, in evaluation order: binding events, reads with the definitions that can reach them, uses and shadows. It selects
//! nothing among them.

pub(crate) mod service;
pub(crate) mod types;

