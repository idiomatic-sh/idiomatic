//! The `template_placing` concept: the files each template entry of a placement's scopes places, and the files a
//! `rules-exclude` covers.

pub mod constants;
pub mod errors;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
