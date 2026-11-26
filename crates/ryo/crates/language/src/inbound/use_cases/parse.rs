//! The `language` domain's `parse` use case.

pub mod collect;
pub mod constants;
pub mod errors;
pub mod interface;
pub mod named;
pub mod types;
pub mod use_case;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;

