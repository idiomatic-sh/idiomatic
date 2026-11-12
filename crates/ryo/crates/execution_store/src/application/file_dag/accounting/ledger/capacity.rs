//! Layout arithmetic for retained allocations. The ledger is the one owner of byte accounting,
//! so every estimate of reserved room is written here and nowhere else.
pub(crate) mod service;
#[cfg(test)]
mod service_test;
