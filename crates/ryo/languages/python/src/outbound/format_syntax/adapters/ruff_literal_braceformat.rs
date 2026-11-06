//! The brace-format grammar adapter: Ruff 0.16.5's `format` parser.

pub(crate) mod grammar;
mod service;
mod types;

#[cfg(test)]
mod grammar_test;
