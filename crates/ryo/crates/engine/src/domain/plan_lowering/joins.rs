//! The `joins` concept: the `join` operation (`join(left=, right=, left-on=[..], right-on=[..], output={name: [side,
//! column]})`) lowered as the select it is: both sources under fixed aliases, each key pair an equality, each output a
//! column of its side.

pub(crate) mod constants;
pub(crate) mod service;

#[cfg(test)]
mod service_test;
