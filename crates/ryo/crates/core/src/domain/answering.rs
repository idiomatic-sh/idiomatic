//! The `core` domain's `answering` concept: which rules have cross-file steps, and how what those steps found joins the findings.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
