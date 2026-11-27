//! The `core` domain's `contract_rendering` concept: what a placed file's template content is rendered with, and the
//! contract the rendered content makes of the file.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
