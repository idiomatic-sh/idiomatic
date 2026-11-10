//! The python language's `parse` contract.

pub(crate) mod constants;
pub(crate) mod contract;
pub(crate) mod readers;
mod service;

#[cfg(test)]
mod contract_test;
