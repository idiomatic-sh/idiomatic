//! The test support of the `execution` port: its contract and its fake.

pub mod contract;
pub mod fake;

mod driver;
pub mod executable;
#[cfg(test)]
pub mod observers;
