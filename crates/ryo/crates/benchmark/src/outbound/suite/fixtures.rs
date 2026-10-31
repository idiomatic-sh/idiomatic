//! The test support of the `suite` port: its contract and its fake.

pub mod contract;
pub mod fake;

// The BDD target links this fixture library; unit tests use the in-memory fake.
#[cfg(not(test))]
pub mod driver;
