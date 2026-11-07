//! The go domain's `links` concept: which files of the repository each package a file imports is, from the imports the parse noted of
//! each file and the modules the manifests declared.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
