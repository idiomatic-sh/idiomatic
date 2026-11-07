//! The go domain's `shapes` concept: what the nodes and the tokens of `go/ast` and `go/token` are in the library's tree of a file. The
//! library's tree is a concrete one, with a node for every keyword, bracket and comma and groups `go/ast` does not make; each node of it
//! is a node of `go/ast` over the same bytes, or none, or several, and each leaf is a token of `go/token`.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
