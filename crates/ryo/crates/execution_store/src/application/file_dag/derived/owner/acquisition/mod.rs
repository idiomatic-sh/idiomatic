//! Sole admitted acquisition and resource-lifetime boundary for every derived kind.
pub mod semantics;
pub(in crate::application::file_dag) mod service;

pub(in crate::application::file_dag::derived::owner) mod scoped;

pub(in crate::application::file_dag) mod population;

pub(in crate::application::file_dag::derived::owner) mod graph;


pub mod admission;

#[cfg(any(test, feature = "fixtures"))]
pub mod family;
