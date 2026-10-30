//! The rust domain's `fact_notes` concept: the facts of one file as the bytes the parse leaves for the project of the same run to read, and the readers and writers of the bytes the notes are made of.

pub(crate) mod service;
#[cfg(test)]
mod service_test;
pub(crate) mod types;
