//! The go domain's `constraints` concept: the build constraint lines of a file, `//go:build` and `// +build`, as `go/build` finds them in
//! the file's header.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
