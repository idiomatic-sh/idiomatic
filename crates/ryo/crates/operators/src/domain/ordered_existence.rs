//! Pure partition extrema and dominated-row flagging; no engine state or retention policy.
pub(crate) mod service;

pub mod types;

#[cfg(test)]
mod service_test;
