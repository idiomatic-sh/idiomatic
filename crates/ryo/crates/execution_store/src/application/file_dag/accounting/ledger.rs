//! Accounting for physical payloads held by derived resource handles.
//! Admission policy, cache selection and scheduling stay with the caller.
mod arithmetic;
pub(crate) mod capacity;
mod constants;
mod credit;
#[cfg(test)]
mod fixtures;
pub(crate) mod service;
#[cfg(test)]
mod service_test;
mod snapshot;
pub mod types;
