//! Preparation never reads rows, emits errors early or chooses retention.
pub mod service;

#[cfg(test)]
mod service_test;
