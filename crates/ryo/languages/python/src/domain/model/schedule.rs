//! The `schedule` part of the model: the file's names in the order the language evaluates them. Values come before the targets
//! they are bound to, a callable's body after the phase that defines it, a generator's body after its first iterable, and the
//! arms of branches and loops are marked as alternatives.

pub(crate) mod service;
pub(crate) mod types;

