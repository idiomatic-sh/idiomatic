//! What the conformance holds the adapter to, which only tests and the `fixtures` feature use: the library's own walk of a tree and
//! its own reading of the items of a file.

#[cfg(test)]
pub(crate) mod model_walk;
pub(crate) mod pinned;
#[cfg(test)]
pub(crate) mod sources;
