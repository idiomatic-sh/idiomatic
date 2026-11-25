//! The `nodes` part of the lowering: the pack's one store of merged plan nodes. A step is named by its digest, whatever rule
//! holds it; the store says what a step's rows look like and how many readers it has across the pack.

pub(crate) mod service;
pub(crate) mod types;

