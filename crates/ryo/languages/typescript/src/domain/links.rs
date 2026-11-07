//! The typescript domain's `links` concept: which file of the repository each module a file imports is, from the imports the parse
//! noted of each file and the project configurations the manifests declared.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
