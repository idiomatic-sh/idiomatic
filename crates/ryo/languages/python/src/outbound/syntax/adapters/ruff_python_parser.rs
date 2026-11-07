//! The syntax port, implemented with `ruff_python_parser`.

pub(crate) mod adapter;
mod annotation;
mod constants;
mod declaration;
#[cfg(any(test, feature = "fixtures"))]
pub(crate) mod fixtures;
mod future;
mod lexical;
#[cfg(test)]
mod lexical_test;
mod options;
mod schemas;
mod scopes;
pub(crate) mod types;

#[cfg(test)]
mod adapter_test;

mod semantic;
#[cfg(test)]
mod semantic_test;
mod writer;

#[cfg(test)]
mod declaration_test;

#[cfg(test)]
mod scopes_test;

#[cfg(test)]
mod annotation_test;

#[cfg(test)]
mod future_test;

#[cfg(test)]
mod options_test;
