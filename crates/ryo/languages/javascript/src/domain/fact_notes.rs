//! The javascript domain's `fact_notes` concept: what the parse leaves of a file for the project to read, the module specifiers it
//! imports. The port `notes` writes them as bytes and reads them back.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
