//! The `validate_store` port: the versions `validate` proved held, kept between runs in the project's store.

pub(crate) mod adapters;
pub mod errors;
pub mod factory;
#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
pub mod interface;
