//! The `shapes` part of the `needs` concept: which of the shapes the needs ask the file's tree to hold (an element under
//! a parent with given kinds and field) the file's node table holds, found from its lists by kind once a file.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
