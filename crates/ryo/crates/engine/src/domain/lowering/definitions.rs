//! The `definitions` part of the lowering: the pack as it is being lowered. A sub-plan is a definition, lowered once and named
//! by its merged plan node; a use of it is a call, and asks one pattern of it.

pub(crate) mod service;
pub(crate) mod types;

