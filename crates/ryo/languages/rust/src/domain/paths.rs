//! The rust domain's `paths` concept: the places of a repository's files, written from its root with `/`.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
