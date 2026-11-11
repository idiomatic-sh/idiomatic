//! Borrowed axis resources exposed by the engine; primitives cannot retain cache handles. The engine's walk
//! over them (a tree of elements, or an index of edges with its domain) is its strategy behind the
//! operators' axis driver port.
pub(crate) mod service;
