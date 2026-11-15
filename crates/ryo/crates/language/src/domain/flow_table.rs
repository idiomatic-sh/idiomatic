//! The `flow_table` concept: the control flow graphs a language's read keeps in a file's element table, written through the table's
//! builder and read back by the readers of `flow-edge`, `flow-block`, `flow-member` and `flow-step`.
pub mod service;
#[cfg(test)]
mod service_test;
