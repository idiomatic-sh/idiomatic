//! The python domain's `lexical_facts` concept: the rows of the relations that state a file's scopes, definitions, references,
//! static resolutions, qualified names and implicit symbols, written from the model.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;

#[cfg(test)]
pub(crate) mod fixtures;
