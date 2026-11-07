//! The `language` domain's `trees` concept: a file's tree written from its library's events and the text between them, so that its
//! tokens spell the whole file.

pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
