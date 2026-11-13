//! Admit a measurement only when its audit holds it to the inventory its scope claims.
#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
pub mod interface;
mod service;
pub mod types;
pub mod use_case;

#[cfg(test)]
mod use_case_test;
