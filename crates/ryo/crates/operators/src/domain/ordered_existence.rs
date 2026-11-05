//! Pure partition extrema and dominated-row flagging; no engine state or retention policy.
pub(crate) mod service;

pub(crate) mod types;

#[cfg(test)]
mod service_test;
