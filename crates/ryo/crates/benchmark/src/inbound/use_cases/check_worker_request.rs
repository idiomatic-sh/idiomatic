//! Refuse a worker request whose identity is not the one its executable was built for.
pub mod interface;
pub mod types;
pub mod use_case;

#[cfg(test)]
mod use_case_test;
