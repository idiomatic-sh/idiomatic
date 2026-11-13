//! The inventory port, implemented with the local filesystem.

pub(crate) mod adapter;
mod authored;
mod canonical;
mod case;
mod case_identity;
mod case_wire;
mod constants;
mod context_identity;
mod difference;
mod discrepancy;
mod evidence;
mod findings;
mod instance;
mod invocation;
mod output;
mod profile;
mod project_inputs;
mod registration;
pub(crate) mod types;

#[cfg(test)]
mod adapter_test;
