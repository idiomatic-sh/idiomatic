//! What the conformance holds the adapter to, which only tests and the `fixtures` feature use: the library's own walk of a tree and
//! its own reading of the rules of a stylesheet.

mod components;
pub(crate) mod oracle;
#[cfg(test)]
pub(super) mod projection;
mod tokens;
pub(crate) mod walk;
