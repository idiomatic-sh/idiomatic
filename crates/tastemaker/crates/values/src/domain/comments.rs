//! The `core` domain's `comments` concept: a file's comments, as the shared `comment` relation gives them, and its lines as
//! they leave them.

pub mod constants;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
