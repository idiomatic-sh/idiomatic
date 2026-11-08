//! Bounded per-generation ownership of immutable compact graph access and successful memos.

#[cfg(test)]
mod nearest;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
pub(super) mod test;
