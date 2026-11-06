//! Read-only owning-vector capacities; no allocator, cache or execution authority.
pub mod value;

#[cfg(test)]
pub(in crate::domain::stores) mod value_test;
