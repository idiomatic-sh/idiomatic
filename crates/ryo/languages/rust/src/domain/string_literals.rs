//! The rust domain's `string_literals` concept: where a file's tree writes a string literal, as the token that holds it: the string
//! of a literal expression, or one written on its own where Rust reads a value (an attribute's arguments, a `cfg` predicate, an
//! ABI).

pub(crate) mod service;

#[cfg(test)]
mod service_test;
