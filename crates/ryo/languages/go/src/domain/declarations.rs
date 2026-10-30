//! The go domain's `declarations` concept: the nodes of `go/ast` a function, a method, a function type, a parameter list and a
//! declaration are made of, which the library's tree does not group the way `go/ast` does.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
