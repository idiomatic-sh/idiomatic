//! The `language` domain's `fillers` concept: what a language implements for the engine, one filler per `Filled` relation, and the
//! fact source the engine reads them through.

pub mod service;
pub mod types;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;

#[cfg(test)]
mod service_test;
