//! The html domain's `bases` concept: what the targets of a document are resolved against, the `<base href>` it names or its own path.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
