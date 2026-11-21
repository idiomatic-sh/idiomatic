//! The python language's `walk` contract: its walk for the engine and the fillers of its relations.

pub(crate) mod contract;
mod fillers;
mod model_fills;
pub(crate) mod source_location;

#[cfg(test)]
mod contract_test;
