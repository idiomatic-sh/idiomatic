//! The `KeyedRows` value: rows keyed by the hash of their key.

#[expect(
    unreachable_pub,
    reason = "the value-module template places `pub mod value`; its parent `keyed_rows` is crate-private, so nothing outside the crate reaches it"
)]
pub mod value;

#[cfg(test)]
mod value_test;
