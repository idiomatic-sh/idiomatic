//! The `language` domain's `fact_tables` concept: the one fact contract between a language and the engine. The engine asks
//! one relation of one file by a request (the relation, the column its rows are found by, presence only or which columns);
//! the language writes the answer into a table through a writer; the engine reads every table through one lookup, whether it
//! holds one row at most a key or a list a key, and whatever wrote it: a parser's walk, a semantic model or a type checker.

pub mod service;
pub mod types;

mod keying;
pub mod reading;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;

#[cfg(test)]
mod service_test;
