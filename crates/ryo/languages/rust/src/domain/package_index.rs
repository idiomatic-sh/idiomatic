//! The rust domain's `package_index` concept: the packages of a repository as extents of directories, and which package holds a file.

pub(crate) mod constants;
pub(crate) mod service;
#[cfg(test)]
mod service_test;
pub(crate) mod types;
