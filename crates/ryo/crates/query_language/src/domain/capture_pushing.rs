//! The `query_language` domain's `capture_pushing` concept: the filters made before the capture they follow, so the capture matches only the rows they let through.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
