//! What the conformance holds the adapter to, which only tests and the `fixtures` feature use: the library's own walk of a tree and
//! its own reading of the items of a file.

#[cfg(test)]
pub(super) mod comparison;
pub(crate) mod oracle;
#[cfg(test)]
pub(super) mod roles;
#[cfg(test)]
pub(super) mod unproduced;
pub(crate) mod walk;
