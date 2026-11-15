//! The rust domain's `shapes` concept: the structural class of every node of a file, over the language kernel's shared key index.
//! Two nodes share a class when their kinds, their children's classes in order, their numeric values and suffixes, their decoded
//! strings and their other tokens are equal; white space and comments are not part of it, and a parenthesized expression, pattern
//! or type is the class of what it holds.

pub(crate) mod service;

#[cfg(test)]
mod service_test;
