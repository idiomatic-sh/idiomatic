//! The `keyed_ranges` concept: the rows a probed step answered for each key in a file, every key's rows one range of its
//! slot's columns. A key that is a row or node number of the file is found by position, any other key through a flat hash;
//! a slot's buffers are kept across files and emptied by resetting lengths.

pub mod constants;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
