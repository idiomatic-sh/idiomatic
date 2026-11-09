//! The `query_language` domain's `refusing` concept: the steps a statement lowered to, refused when they cannot run where they are placed; a span states no file, so a step comparing rows on spans alone runs only inside one file, and a walk keeps its vertices of one kind.
#[cfg(test)]
pub(crate) mod fixtures;
pub(crate) mod service;

#[cfg(test)]
mod service_test;
