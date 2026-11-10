//! A corpus is identified by its digest; its comparison's differences are accepted only by finding, for that corpus.
mod constants;
pub(crate) mod errors;
pub(crate) mod service;
#[cfg(test)]
mod service_test;
pub(crate) mod types;
