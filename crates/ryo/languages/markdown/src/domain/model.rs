//! The markdown domain's `model` concept: the sections a file's headings open, which nest as the headings' levels do.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
