//! The go domain's `pieces` concept: the nodes of `go/ast` and the tokens of `go/token` found in the library's tree, before they are
//! written into the table in the order of the file.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
