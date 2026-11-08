//! Sole admitted acquisition and resource-lifetime boundary for every derived kind.
pub(crate) mod semantics;
pub(crate) mod service;

pub(crate) mod scoped;

pub(crate) mod population;

pub(crate) mod graph;


pub(crate) mod admission;

#[cfg(test)]
pub(crate) mod family;
