//! The `language` domain's `facts` concept: the fact contract the engine reads a language's relations of one file through, and the
//! words of a file.

pub mod errors;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
