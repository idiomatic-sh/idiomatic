//! Sole admitted acquisition and resource-lifetime boundary for every derived kind.
pub(in crate::application::file_dag) mod semantics;
pub(in crate::application::file_dag) mod service;

pub(in crate::application::file_dag::derived::owner) mod graph;

pub(in crate::application::file_dag::derived) mod selection;
