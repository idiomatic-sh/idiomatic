//! The rust domain's `attribute_paths` concept: the paths a file's attributes write, a meta's path node or a run of tokens in an
//! attribute's token tree, each with the name it is written as.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
