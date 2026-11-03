//! The syntax port, implemented with `ruff_python_parser`.

pub(crate) mod adapter;
mod constants;
#[cfg(any(test, feature = "fixtures"))]
pub(crate) mod fixtures;
mod lexical;
#[cfg(test)]
mod lexical_test;
mod schemas;
pub(crate) mod types;

#[cfg(test)]
mod adapter_test;

mod writer;
mod semantic;

#[cfg(test)]
mod declaration_test;
