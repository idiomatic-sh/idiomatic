//! The `query_language` domain's `defining` concept: one definition of a linked program lowered into the plan under
//! construction, with the fragment contracts it starts or returns checked, and a refusal placed in its document.

mod constants;
pub(crate) mod service;

#[cfg(test)]
mod service_test;
