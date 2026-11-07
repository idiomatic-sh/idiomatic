//! The `language` domain's `targets` concept: what a target a file writes (a link, an import, a url) names, as a path of the run.

mod constants;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
