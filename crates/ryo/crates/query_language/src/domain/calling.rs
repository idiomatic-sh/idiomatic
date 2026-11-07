//! The `query_language` domain's `calling` concept: the calls that shape rows lowered to steps: `project`, `extend`, `join` and `antijoin`, and the dispatch of every call.

pub(crate) mod service;

#[cfg(test)]
mod service_test;
