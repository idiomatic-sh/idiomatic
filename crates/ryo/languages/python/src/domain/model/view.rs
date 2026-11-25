//! The `view` part of the model: the whole model of one file, and the per-file view as the semantic fillers read it. The model is
//! built by the first fill that asks, the lexical part alone or with the flow part, and never by the walk.

pub(crate) mod service;
pub(crate) mod types;

