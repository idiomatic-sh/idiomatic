//! The go domain's `roles` concept: the role of each element of the tree under its parent, in the words of `go/ast`: the field of its
//! node that holds it.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;

#[cfg(test)]
mod fixtures;
