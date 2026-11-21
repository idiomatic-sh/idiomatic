//! The `executing` concept: the one loop over a program's words.

pub mod asks;
pub mod bodies;
pub mod budget;
pub mod closing;
pub mod constants;
pub mod entries;
#[cfg(test)]
pub mod fixtures;
pub mod kept;
pub mod meanings;
pub mod measuring;
pub mod ordered;
pub mod presence;
pub mod running;
pub mod service;
pub mod sources;
pub mod staging;
pub mod types;
pub mod walks;

#[cfg(test)]
mod service_test;
