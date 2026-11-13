//! The typescript domain's `shapes` concept: the structural class of every node of a file, over the language kernel's shared key
//! index. Two nodes share a class when their kinds, their children's classes in order, their numeric values, their decoded
//! names and strings and their other tokens are equal; comments and the parentheses the tree keeps as punctuators are not part of
//! it.

pub(crate) mod service;

#[cfg(test)]
mod service_test;
