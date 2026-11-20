//! The `reaching` part of the model: which definitions of a name can reach a point of the schedule.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
