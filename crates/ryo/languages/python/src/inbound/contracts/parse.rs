//! The python language's `parse` contract.

pub(crate) mod brace_format_readers;
pub(crate) mod contract;
pub(crate) mod format_readers;
mod format_specification_readers;
pub(crate) mod raw_format_readers;

#[cfg(test)]
mod contract_test;

pub(crate) mod literal_expression_readers;
