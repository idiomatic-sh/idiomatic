//! The rust domain's `workspaces` concept: which workspace holds a package, and which packages a workspace holds.

pub(crate) mod constants;
pub(crate) mod errors;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(any(test, feature = "fixtures"))]
pub(crate) mod fixtures;

#[cfg(test)]
mod service_test;
