//! Sole admitted acquisition and resource-lifetime boundary for every derived kind.
pub mod semantics;
pub(crate) mod service;

pub(crate) mod scoped;

pub(crate) mod population;

pub(crate) mod graph;


pub mod admission;

#[cfg(any(test, feature = "fixtures"))]
pub mod family;
