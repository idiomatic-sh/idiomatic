//! The `budget` part of the `executing` concept: what a rule is charged in a file, and the notice when it passes its
//! allowance.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
