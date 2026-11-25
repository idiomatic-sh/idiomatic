//! Portable worklist semantics, first in first out: a state for each entity from a declared lattice, reached by a worklist
//! that follows a source's edges in order, and the trace of every edge it follows.
pub(crate) mod service;
pub(crate) mod types;

