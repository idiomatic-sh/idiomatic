//! A closed semantic service: program and schema are fixed by request preparation.
//! Matcher acquisition, short circuit and cache lifetime stay inside this owner.
#[cfg(test)]
mod constants;
#[cfg(test)]
mod service_test;
pub(crate) mod types;
