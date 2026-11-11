//! Native lookup is immutable semantic data, with no cache or service capabilities; the engine's join
//! candidates (an index probe, or the elements of a tree a left row finds) are its strategy behind the
//! operators' join driver port.
pub(crate) mod service;
