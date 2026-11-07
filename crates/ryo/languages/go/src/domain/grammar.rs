//! The go domain's `grammar` concept: the parser library's tree of a file as plain data, in the words of Go's own syntax, and how to
//! move about it. The library's names stop at the adapter; what it hands over is what each of its nodes is in Go.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
