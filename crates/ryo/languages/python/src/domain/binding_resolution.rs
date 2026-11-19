//! Schedule-order name resolution: for each read or deletion, the event in effect in each namespace on its lookup path.
pub(crate) mod service;
#[cfg(test)]
mod service_test;
pub(crate) mod types;
