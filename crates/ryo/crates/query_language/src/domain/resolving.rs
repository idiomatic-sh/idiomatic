//! The `query_language` domain's `resolving` concept: where the names a query writes become what they name, and where the arguments of a call are read.

pub(crate) mod service;

#[cfg(test)]
mod service_test;
