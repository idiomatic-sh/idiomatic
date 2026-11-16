//! A worker request run inside this process through the language's worker; every other invocation is refused.
pub(crate) mod adapter;

#[cfg(test)]
mod adapter_test;
mod constants;
