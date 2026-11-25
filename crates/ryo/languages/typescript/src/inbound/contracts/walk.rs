//! The typescript language's `walk` contract.

#![cfg_attr(not(test), allow(dead_code, reason = "the walk is linked when step 0 of the ports lands; its tests read it until then"))]

pub(crate) mod contract;
mod fillers;

#[cfg(test)]
mod contract_test;
