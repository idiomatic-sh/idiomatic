//! The `RowSet` value: rows kept once, each by its hash and its row, with no link to another row.

#[expect(
    unreachable_pub,
    reason = "the value-module template places `pub mod value`; its parent `row_set` is crate-private, so nothing outside the crate reaches it"
)]
pub mod value;

#[cfg(test)]
mod value_test;
