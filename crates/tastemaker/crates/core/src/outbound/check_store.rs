//! The `check_store` port: the answer `check` gave and the files it rests on, kept between runs in the project's store.

pub(crate) mod adapters;
pub mod errors;
pub mod factory;
#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
pub mod interface;
pub mod types;
