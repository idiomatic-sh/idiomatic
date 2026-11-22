//! The `counts` pass (`pass_count.py`): a two-source select whose items read one source only, whose one reader is a
//! `group-count` of it without `distinct`, and which no rule emits, is marked to be answered by counts of the probed
//! rows of each kept row.

pub(crate) mod service;

#[cfg(test)]
mod service_test;
