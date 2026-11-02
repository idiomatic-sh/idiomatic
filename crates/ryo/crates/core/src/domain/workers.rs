//! The `core` domain's `workers` concept: what each worker of a run owns while the files are visited, what every visit reads in
//! common, and how what the workers made meets once they are done.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;
