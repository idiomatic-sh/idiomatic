//! This process's clock and its `/proc` memory figures, on Linux.
pub(crate) mod adapter;
#[cfg(all(test, target_os = "linux"))]
mod adapter_test;
mod constants;
