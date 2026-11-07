//! The go domain's `modfile` concept: a module's `go.mod` read into the table line by line, as `golang.org/x/mod/modfile` reads it.
//! A `go.mod` is no go source and no parser library reads it: the language reads it itself.

mod constants;
pub(crate) mod service;
mod types;

#[cfg(test)]
mod service_test;
