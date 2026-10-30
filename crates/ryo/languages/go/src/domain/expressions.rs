//! The go domain's `expressions` concept: the nodes of `go/ast` a union, an instantiation, a call, a conversion, a unary expression, an
//! interface, a field and a composite literal are made of, which the library's tree does not group the way `go/ast` does.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
