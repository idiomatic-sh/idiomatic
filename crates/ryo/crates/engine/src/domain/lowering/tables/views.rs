//! The `views` part of the tables: a step that is one read of a relation with its pins. Such a step is no table: each
//! reader that takes its branches reads the relation itself, with the step's pins.

pub(crate) mod service;
pub(crate) mod types;
