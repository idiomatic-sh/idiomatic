//! The syntax port's implementations, one module each.

pub(super) mod ruff_python_parser;

#[cfg(feature = "fixtures")]
pub(crate) use ruff_python_parser::{visit_parsed, walk_text};
