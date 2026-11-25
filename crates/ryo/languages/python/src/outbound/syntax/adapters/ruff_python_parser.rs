//! The syntax port, implemented with `ruff_python_parser`.

pub(crate) mod adapter;
mod annotation;
mod constants;
mod declaration;
pub(crate) mod fixtures;
mod future;
mod lexical;
mod options;
mod schemas;
mod scopes;
pub(crate) mod types;
mod embedded;
mod walk;
mod walker;


mod semantic;
mod writer;






