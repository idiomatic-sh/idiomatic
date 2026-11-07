//! The python domain's `definitions` concept: what an element defines, as the `defines` and `parameter` relations say it: a
//! function, a class, an imported or assigned name, a parameter and its place, an exception's name, a pattern's capture or a type
//! parameter, with Python's own word for each.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
