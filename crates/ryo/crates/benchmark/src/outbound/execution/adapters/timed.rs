//! Fresh-process resource isolation.
#[cfg(not(test))]
pub(crate) mod adapter;
#[cfg(not(test))]
mod constants;
mod schemas;
#[cfg(not(test))]
pub(crate) mod settings;
