//! The markdown domain's `labels` concept: the names a document gives its places in raw HTML, which a link can point at.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
