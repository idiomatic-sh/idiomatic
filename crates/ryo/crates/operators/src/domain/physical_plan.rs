//! The physical plan of a whole program: each node's kind lowered once into kernels, the rewrites its consumers' demand allows,
//! pipelines broken only at blocking kernels and shared nodes, and the kernels run over one file's buffers.

pub(crate) mod service;
#[cfg(test)]
mod service_test;
pub mod types;
