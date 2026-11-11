//! Immutable engine-prepared graph inputs and invocation-bounded traversal work. The engine's walks over its
//! closure partitions and its ordered depth-first graph, and its prepared adjacency and ordered keys, are
//! its strategies behind the operators' closure, depth-first, cycle and ordered-match driver ports.
pub(crate) mod service;
