//! The `language` domain's `keys` concept: a file's structural keys, each complete key interned once, so that equal keys share
//! the first occurrence that had them. Hashes choose candidates; complete word slices decide equality.

pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
