//! The engine's side of one evaluation: the lease, the downstream sink and the prepared inputs behind the
//! operators' driver ports. Every kind's row meaning is the operators'; each strategy the engine keeps (an
//! index probe, native tree candidates, its walks over trees, partitions and graphs) implements one port.
pub(crate) mod service;
pub(crate) mod types;
