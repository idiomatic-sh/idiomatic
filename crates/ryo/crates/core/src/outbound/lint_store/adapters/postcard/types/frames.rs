//! Streaming the existing postcard bucket framing without rehydrating payloads.

pub(in crate::outbound::lint_store::adapters::postcard) mod value;

#[cfg(test)]
mod value_test;
