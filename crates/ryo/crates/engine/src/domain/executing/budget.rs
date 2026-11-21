//! The `budget` part of the `executing` concept: what a rule is charged in a file, and the notice when it passes its
//! allowance.

pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
