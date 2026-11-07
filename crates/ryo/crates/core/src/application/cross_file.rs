//! The `cross_file` application operation.
pub(crate) mod completion;
pub(crate) mod constants;
pub(crate) mod errors;
#[cfg(test)]
mod fixtures;
pub(crate) mod ordinary;
pub(crate) mod preparation;
pub(crate) mod publication;
#[cfg(test)]
mod publication_test;
pub(crate) mod rendering;
pub mod service;
#[cfg(test)]
mod service_test;
pub(crate) mod types;
