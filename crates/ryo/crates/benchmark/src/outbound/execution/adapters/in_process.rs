//! A worker's request run inside this process over the edge's engine; every other invocation runs as a child.
pub(crate) mod adapter;

#[cfg(test)]
mod adapter_test;
mod constants;
