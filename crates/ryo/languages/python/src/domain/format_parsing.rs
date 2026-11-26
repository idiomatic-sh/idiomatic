//! Parse decoded format strings lazily and retain results once per distinct input.
pub(crate) mod service;


pub(crate) mod constants;

#[cfg(test)]
mod service_test;
