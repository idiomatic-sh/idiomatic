//! The `semijoins` pass (`pass_semi.py`): a two-source select whose items read one source only (the kept one) and whose
//! other source (the probed one) appears only in its tests, at least one of which reads both, is marked a semijoin of
//! the probed source when no reader can tell a row of it from its copies.

pub(crate) mod service;

#[cfg(test)]
mod service_test;
