//! Parse decoded format strings lazily and retain results once per distinct input.
pub(crate) mod service;

#[cfg(test)]
mod service_test;
