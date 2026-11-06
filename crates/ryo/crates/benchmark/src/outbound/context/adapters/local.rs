//! Local snapshot validation and its private wire schema.
pub(crate) mod adapter;
#[cfg(test)]
mod adapter_test;
mod constants;
pub(crate) mod schemas;
#[cfg(test)]
mod schemas_test;
