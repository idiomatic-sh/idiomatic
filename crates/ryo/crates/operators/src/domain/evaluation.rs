//! One evaluation of a plan operation: every kind's row meaning over the engine's driver port.
pub(crate) mod service;
pub mod types;

#[cfg(test)]
pub(crate) mod fixtures;
#[cfg(test)]
mod service_test;
