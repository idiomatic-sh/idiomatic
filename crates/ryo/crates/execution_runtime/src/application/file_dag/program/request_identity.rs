//! One immutable listed corpus and execution generation, minted by composition
//! once per actual run. Prepared query programs may be reused; this identity may not.
pub(crate) mod service;
#[cfg(test)]
mod service_test;
pub(crate) mod types;
