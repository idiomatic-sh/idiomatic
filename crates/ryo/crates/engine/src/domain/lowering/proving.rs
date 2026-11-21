//! The `proving` part of the lowering: the invariant every lowered pack holds before it is answered. A file-sized item stands
//! only as a rule branch's first step, as a pipeline's first stage, or as a product stage of a keyless join's own table.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
