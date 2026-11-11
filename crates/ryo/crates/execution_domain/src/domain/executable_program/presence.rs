//! The executable program's `presence` concept: what one file must hold for a node of the request graph to have any row, so a
//! rule that cannot match a file is never instantiated there.
mod narrowing;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
