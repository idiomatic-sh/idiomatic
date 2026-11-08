//! Request-prepared source classes and ordered predicate prefixes over admitted
//! file populations. Application owns readiness, sharing scope and result leases.

#[cfg(test)]
mod publication;
pub mod service;
#[cfg(test)]
pub(crate) mod service_test;
pub mod types;
