//! Sole admitted acquisition and resource-lifetime boundary for every derived kind.
pub(in crate::application::file_dag) mod semantics;
pub(in crate::application::file_dag) mod service;

pub(in crate::application::file_dag::derived::owner) mod scoped;

pub(in crate::application::file_dag::derived::owner) mod population;

pub(in crate::application::file_dag::derived::owner) mod graph;

pub(in crate::application::file_dag::derived::owner) mod selection;

pub(in crate::application::file_dag) mod admission;

#[cfg(test)]
pub(in crate::application::file_dag) mod family;
